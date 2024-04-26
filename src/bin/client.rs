use clap::{Parser, Subcommand};
use coin::io::FileIO;
use coin::keys::KeyPair;
use std::fs::File;
use std::path::Path;

#[derive(Parser)]
#[command(name = "Coin")]
#[command(version = "0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    GenKeys { name: String },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenKeys { name } => {
            let key_pair = KeyPair::new();
            let file_name = format!("{}.key", name);
            let path = Path::new(&file_name);

            match File::create_new(path) {
                Ok(mut file) => {
                    key_pair
                        .to_file(&mut file)
                        .expect("Failed to save keys to file.");
                }
                Err(_) => {
                    println!("Failed to open file");
                }
            }
        }
    }
}
