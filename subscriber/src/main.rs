use clap::Parser;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use std::{collections::HashMap, error::Error, fs, time::Duration};
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
};

#[derive(Deserialize)]
struct Config {
    geyser_url: String,
    token: String,
    pool_address: String,
    wallet_private_key: String,
    recipient_address: String,
}

#[derive(Parser)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long)]
    config: String,
}

type AccountFilterMap = HashMap<String, SubscribeRequestFilterAccounts>;

#[derive(Debug, Clone, Parser)]
#[clap(author, version, about)]
struct BlockService {
    geyser_url: String,
    token: String,
    pool_address: String,
}

impl BlockService {
    fn new(geyser_url: String, token: String, pool_address: String) -> Self {
        Self {
            geyser_url,
            token,
            pool_address,
        }
    }

    async fn connect(&self) -> Result<GeyserGrpcClient<impl Interceptor>, Box<dyn Error>> {
        GeyserGrpcClient::build_from_shared(self.geyser_url.clone())?
            .x_token(Some(self.token.clone()))?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .max_decoding_message_size(1024 * 1024 * 1024)
            .connect()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    fn get_pool_subsribe_request(&self) -> Result<SubscribeRequest, Box<dyn std::error::Error>> {
        let mut accounts: AccountFilterMap = HashMap::new();

        accounts.insert(
            "client".to_owned(),
            SubscribeRequestFilterAccounts {
                nonempty_txn_signature: None,
                account: vec![self.pool_address.to_string()],
                owner: vec![],
                filters: vec![],
            },
        );

        Ok(SubscribeRequest {
            from_slot: Some(0),
            slots: HashMap::default(),
            accounts,
            transactions: HashMap::default(),
            transactions_status: HashMap::default(),
            entry: HashMap::default(),
            blocks: HashMap::default(),
            blocks_meta: HashMap::default(),
            commitment: Some(CommitmentLevel::Processed as i32),
            accounts_data_slice: Vec::default(),
            ping: None,
        })
    }
}

async fn send_transaction(
    wallet: &Keypair,
    recipient_address: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a Solana RPC client
    let solana_client = RpcClient::new("https://api.devnet.solana.com");

    // Get the latest blockhash
    let recent_blockhash = solana_client.get_latest_blockhash()?;

    // Create the transaction instruction to transfer SOL
    let transfer_instruction = solana_sdk::system_instruction::transfer(
        &wallet.pubkey(),
        &recipient_address.parse::<Pubkey>()?, // Convert the recipient address to Pubkey
        1_000_000_000, // Amount to send in lamports (1 SOL = 1_000_000_000 lamports)
    );

    // Create and sign the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&wallet.pubkey()),
        &[wallet],
        recent_blockhash,
    );

    // Send the transaction and wait for confirmation
    let signature = solana_client.send_and_confirm_transaction(&transaction)?;

    println!(
        "Transaction sent to {} with signature: {}",
        recipient_address, signature
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration
    let config: Config = serde_yaml::from_str(&fs::read_to_string(cli.config)?)?;

    // create client
    let block_service = BlockService::new(config.geyser_url, config.token, config.pool_address);
    let mut client = block_service.connect().await?;

    // Subscribe to block events
    let request = block_service.get_pool_subsribe_request()?;
    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;

    let wallet = Keypair::from_base58_string(&config.wallet_private_key);
    let recipient_address = config.recipient_address;

    while let Some(_) = stream.next().await {
        send_transaction(&wallet, &recipient_address).await?;
    }

    Ok(())
}
