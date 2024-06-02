use clap::{Parser, Subcommand};
use coin::core::keys::{KeyPair, PublicKey};
use coin::core::transaction::{Output, Value};
use coin::traits::io::FileIO;
use coin::utils::{json_to_utxos, new_tx};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "Coin")]
#[command(version = "0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    #[command(about = "Generate new key pair")]
    GenKeys { path: PathBuf },

    #[command(about = "Print public address for key")]
    GetAddr { path: PathBuf },

    #[command(about = "Print the number of coins the key owns")]
    GetFunds {
        #[arg(short, long, required = false, default_value = "http://127.0.0.1:8080")]
        node: String,
        path: PathBuf,
    },

    #[command(about = "Send coins to address")]
    Send {
        #[arg(short, long, required = false, default_value = "http://127.0.0.1:8080")]
        node: String,
        key: PathBuf,
        addr: String,
        value: Value,
    },

    #[command(about = "Build transaction")]
    BuildTx {
        tx: PathBuf,
        addr: String,
        value: Value,
    },

    #[command(about = "Send transaction")]
    SendTx {
        #[arg(short, long, required = false, default_value = "http://127.0.0.1:8080")]
        node: String,
        key: PathBuf,
        tx: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenKeys { path } => {
            println!("Saving new key pair in file {}", path.display());
            let key = KeyPair::new();
            match key.to_file(path) {
                Ok(_) => println!("Key pair saved!"),
                Err(_) => {
                    println!("Failed to save keys to file");
                    return ExitCode::from(1);
                }
            }
            ExitCode::from(0)
        }
        Commands::GetAddr { path } => {
            let key = match KeyPair::from_file(path) {
                Ok(key) => key,
                Err(_) => {
                    println!("Failed to read key from file!");
                    return ExitCode::from(1);
                }
            };

            println!(
                "Public address for key {}: {}",
                path.to_str().unwrap(),
                key.public_key().to_hex_str()
            );
            ExitCode::from(0)
        }
        Commands::GetFunds { node, path } => {
            let key = match KeyPair::from_file(path) {
                Ok(key) => key,
                Err(_) => {
                    println!("Failed to read key from file!");
                    return ExitCode::from(1);
                }
            };

            let addr = key.public_key().to_hex_str();

            let utxos = match reqwest::blocking::get(format!("{}/utxos/{}", node, addr,)) {
                Ok(body) => json_to_utxos(&body.text().unwrap()).unwrap(),
                Err(err) => {
                    println!("Failed to fetch utxos! {:?}", err);
                    return ExitCode::from(1);
                }
            };

            let value = utxos.into_iter().fold(0, |acc, utxo| acc + utxo.value);
            println!("Total coins for address {}: {}", addr, value);
            ExitCode::from(0)
        }
        Commands::Send {
            node,
            key,
            addr,
            value,
        } => {
            println!("Sending {} coins to {}", value, addr);

            let key = match KeyPair::from_file(key) {
                Ok(key) => key,
                Err(_) => {
                    println!("Failed to read key from file!");
                    return ExitCode::from(1);
                }
            };

            let recipient = match PublicKey::from_hex_str(addr) {
                Ok(key) => key,
                Err(_) => {
                    println!("The address is not valid!");
                    return ExitCode::from(1);
                }
            };

            let outputs = [Output {
                pubkey: recipient,
                value: *value,
            }];

            send_tx(node, key, &outputs)
        }
        Commands::BuildTx { tx, addr, value } => {
            println!("Build transaction into file {}", tx.display());
            println!("  Send {} coins to {}", value, addr);

            let recipient = match PublicKey::from_hex_str(addr) {
                Ok(key) => key,
                Err(_) => {
                    println!("The address is not valid!");
                    return ExitCode::from(1);
                }
            };

            let mut outputs = match Vec::from_file(tx) {
                Ok(outputs) => outputs,
                Err(_) => {
                    println!("Error while reading tx file: {}!", tx.display());
                    println!("Creating a new file.");
                    vec![]
                }
            };

            outputs.push(Output {
                pubkey: recipient,
                value: *value,
            });

            match outputs.to_file(tx) {
                Ok(_) => ExitCode::from(0),
                Err(_) => {
                    println!("Error while saving tx file: {}!", tx.display());
                    return ExitCode::from(1);
                }
            }
        }
        Commands::SendTx { node, key, tx } => {
            println!("Send transaction from file {}", tx.display());

            let key = match KeyPair::from_file(key) {
                Ok(key) => key,
                Err(_) => {
                    println!("Failed to read key from file!");
                    return ExitCode::from(1);
                }
            };

            let outputs = match Vec::from_file(tx) {
                Ok(outputs) => outputs,
                Err(_) => {
                    println!("Error reading tx file {}!", tx.display());
                    return ExitCode::from(1);
                }
            };

            send_tx(node, key, outputs.as_slice())
        }
    }
}

fn send_tx(node: &str, key: KeyPair, outputs: &[Output]) -> ExitCode {
    let mut utxos =
        match reqwest::blocking::get(format!("{}/utxos/{}", node, key.public_key().to_hex_str())) {
            Ok(body) => json_to_utxos(&body.text().unwrap()).unwrap(),
            Err(err) => {
                println!("Failed to fetch utxos! {:?}", err);
                return ExitCode::from(1);
            }
        };

    utxos.sort_by(|a, b| a.value.cmp(&b.value));
    let tx = match new_tx(&key, &utxos, outputs.to_vec()) {
        Ok(tx) => tx,
        Err(err) => {
            println!("Failed to build transaction: {}", err);
            return ExitCode::from(1);
        }
    };

    println!("Sending transaction: \n{:#?}", tx);

    let client = reqwest::blocking::Client::new();
    match client.post(format!("{}/chain", node)).json(&tx).send() {
        Ok(res) => {
            println!("Response: {}", res.status())
        }
        Err(err) => {
            println!("Failed to send transaction: {:?}", err);
            return ExitCode::from(1);
        }
    };

    ExitCode::from(0)
}
