use ethers::prelude::*;
use futures::future::join_all;
use std::convert::TryFrom;
use std::error::Error;
use std::io::{self, Write};

#[derive(Debug)]
struct ChainFootprint {
    chain_name: String,
    rpc_url: String,
    native_balance: Option<U256>,
    active: bool,
}

async fn probe_network(
    chain_name: String,
    rpc_url: String,
    wallet: Address,
) -> ChainFootprint {
    if let Ok(provider) = Provider::<Http>::try_from(&rpc_url) {
        match provider.get_balance(wallet, None).await {
            Ok(balance) => ChainFootprint {
                chain_name,
                rpc_url,
                native_balance: Some(balance),
                active: !balance.is_zero(),
            },
            Err(_) => ChainFootprint {
                chain_name,
                rpc_url,
                native_balance: None,
                active: false,
            },
        }
    } else {
        ChainFootprint {
            chain_name,
            rpc_url,
            native_balance: None,
            active: false,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Loading environment variables from the .env file
    dotenvy::dotenv().ok();

    // 2. Reading the URL from the configuration
    let rpc_ethereum = std::env::var("RPC_ETHEREUM")?;
    let rpc_bsc = std::env::var("RPC_BSC")?;
    let rpc_polygon = std::env::var("RPC_POLYGON")?;
    let rpc_arbitrum = std::env::var("RPC_ARBITRUM")?;
    let rpc_base = std::env::var("RPC_BASE")?;

    let networks = vec![
        ("Ethereum Mainnet".to_string(), rpc_ethereum),
        ("Binance Smart Chain".to_string(), rpc_bsc),
        ("Polygon PoS".to_string(), rpc_polygon),
        ("Arbitrum One".to_string(), rpc_arbitrum),
        ("Base".to_string(), rpc_base),
    ];

    // 3. Requesting an address from the operator
    print!("Enter wallet address to scan: ");
    io::stdout().flush()?; // Clear the buffer so that the text is displayed before waiting for input

    let mut input_address = String::new();
    io::stdin().read_line(&mut input_address)?;

    // Cleaning input by removing line breaks (\n, \r) and extra spaces
    let input_address = input_address.trim();

    // Checking the address for validity
    let target_wallet: Address = match input_address.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("Error: Invalid Ethereum address format.");
            return Ok(());
        }
    };

    println!("Scanning multichain footprint for wallet: {:?}", target_wallet);
    println!("Probing {} networks in parallel...\n", networks.len());

    // 4. Creating parallel tasks for each network
    let mut tasks = vec![];
    for (name, url) in networks {
        tasks.push(probe_network(name, url, target_wallet));
    }

    let results = join_all(tasks).await;

    // 5. Outputting the results
    println!("--- SCAN RESULTS ---");
    let mut active_count = 0;
    for result in results {
        if result.active {
            active_count += 1;
            let balance_formatted = match result.native_balance {
                Some(bal) => format!("{:.4} ETH/Native", ethers::utils::format_units(bal, "ether")?),
                None => "Unknown".to_string(),
            };
            println!(
                "+ [ACTIVE] {}\n    └─ Balance: {}\n    └─ RPC Used: {}",
                result.chain_name, balance_formatted, result.rpc_url
            );
        } else {
            println!("- [INACTIVE] {}", result.chain_name);
        }
    }

    if active_count == 0 {
        println!("\nNo active balances found on probed EVM chains. The wallet might be new or TRON-only.");
    }

    Ok(())
}
