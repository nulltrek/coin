use clap::{Parser, Subcommand};
use coin::core::keys::KeyPair;
use coin::core::transaction::Value;
use coin::traits::io::FileIO;
use std::fs::File;
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

    #[command(about = "Send coins to address")]
    Send { addr: String, value: Value },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenKeys { path } => {
            println!("Saving new key pair in file {}", path.display());
            match File::create_new(path) {
                Ok(mut file) => {
                    let key = KeyPair::new();
                    println!("{:?}", key.private_key());
                    key.to_file(&mut file)
                        .expect("Failed to save keys to file.");
                    ExitCode::from(0)
                }
                Err(error) => {
                    println!("Failure: {}", error);
                    ExitCode::from(1)
                }
            }
        }
        Commands::GetAddr { path } => {
            let mut key_file = match File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    println!("Failed to open key file: {}", err);
                    return ExitCode::from(1);
                }
            };

            let key = match KeyPair::from_file(&mut key_file) {
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
        Commands::Send { addr, value } => {
            println!("Sending {} coins to {}", value, addr);
            println!("NOT YET IMPLEMENTED");
            ExitCode::from(1)
        }
    }
}
