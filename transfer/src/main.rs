use clap::Parser;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::task;

#[derive(Deserialize)]
struct Config {
    wallets: Vec<String>,
    recipients: Vec<String>,
    amount: u64,
}

#[derive(Parser)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long)]
    config: String,
}

async fn get_latest_blockhash(
    rpc_client: &RpcClient,
) -> Result<solana_sdk::hash::Hash, Box<dyn std::error::Error>> {
    let mut attempts = 0;
    let max_attempts = 5;
    let delay = Duration::from_secs(2);

    while attempts < max_attempts {
        match rpc_client.get_latest_blockhash() {
            Ok(blockhash) => return Ok(blockhash),
            Err(e) => {
                println!("Attempt {} failed: {:?}", attempts + 1, e);
                attempts += 1;
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err("Failed to get latest blockhash after several attempts".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration
    let config: Config = serde_yaml::from_str(&fs::read_to_string(cli.config)?)?;

    // Create an RPC client for the Solana Devnet
    let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com"));

    // Create tasks for sending SOL
    let mut handles = vec![];
    let now = Instant::now();

    // Get the recent blockhash with retry logic
    let recent_blockhash = get_latest_blockhash(&rpc_client).await?;

    for (wallet_key, recipient) in config.wallets.iter().zip(config.recipients.iter()) {
        let wallet = Keypair::from_base58_string(wallet_key);
        let recipient_pubkey = recipient.parse().expect("Invalid recipient address");

        // Clone the Arc to pass to the async task
        let rpc_client_clone = Arc::clone(&rpc_client);
        let recent_blockhash_clone = recent_blockhash;

        let handle = task::spawn(async move {
            let transaction = Transaction::new_signed_with_payer(
                &[system_instruction::transfer(
                    &wallet.pubkey(),
                    &recipient_pubkey,
                    config.amount,
                )],
                Some(&wallet.pubkey()),
                &[&wallet],
                recent_blockhash_clone,
            );

            // Send the transaction and get the signature
            let signature = rpc_client_clone.send_and_confirm_transaction(&transaction);

            // Return the signature
            signature
                .map(|sig| sig.to_string())
                .unwrap_or_else(|_| "Failed to send transaction".to_string())
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete and collect results
    let mut results = vec![];
    for handle in handles {
        let result = handle.await?;
        results.push(result);
    }

    let duration = now.elapsed();
    println!("All transactions sent in: {:?}", duration);

    // Output transaction hashes
    for (i, signature) in results.iter().enumerate() {
        println!("Transaction {}: {}", i + 1, signature);
    }

    Ok(())
}
