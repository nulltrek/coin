use clap::{Parser, Subcommand};
use coin::keys::KeyPair;
use std::path::Path;
use std::fs::File;
use std::io::{BufWriter, Write};

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
                Ok(file) => {
                    let mut buf_writer = BufWriter::new(file);
                    let _ = buf_writer.write(key_pair.serialize().as_slice());
                    let _ = buf_writer.flush();
                },
                Err(_) => {
                    println!("Failed to open file");
                }
            }
        }
    }
}
