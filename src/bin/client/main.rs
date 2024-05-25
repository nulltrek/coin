use clap::{Parser, Subcommand};
use coin::core::keys::KeyPair;
use coin::traits::io::FileIO;
use std::fs::File;
use std::path::PathBuf;

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
}

fn main() {
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
                }
                Err(error) => {
                    println!("Failure: {}", error);
                }
            }
        }
    }
}
