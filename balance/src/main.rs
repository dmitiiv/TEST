use futures::future;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;
use tokio::task;

#[derive(Debug, Deserialize)]
struct Config {
    wallets: Vec<String>,
}

async fn get_balance(wallet_address: &str) -> (String, u64) {
    let client = RpcClient::new("https://api.devnet.solana.com");
    let pubkey = Pubkey::from_str(wallet_address).unwrap();
    let balance = client.get_balance(&pubkey).unwrap();
    (wallet_address.to_string(), balance)
}

#[tokio::main]
async fn main() {
    // Load configuration from config.yaml
    let config_content =
        fs::read_to_string("balance/src/config.yaml").expect("Unable to read config.yaml");
    let config: Config = serde_yaml::from_str(&config_content).expect("Unable to parse YAML");

    let mut tasks = vec![];

    // Create tasks for fetching balances
    for wallet in config.wallets {
        tasks.push(task::spawn(async move { get_balance(&wallet).await }));
    }

    // Collect results
    let results = future::join_all(tasks).await;

    // Print the results
    for result in results {
        match result {
            Ok(balance_info) => {
                let (wallet, balance) = balance_info;
                println!("Wallet: {}, Balance: {} SOL", wallet, balance);
            }
            Err(e) => {
                eprintln!("Error fetching balance: {:?}", e);
            }
        }
    }
}
