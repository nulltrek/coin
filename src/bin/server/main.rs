use clap::{Parser, Subcommand};
use coin::chain::{Chain, SerializableChain};
use coin::traits::io::FileIO;
use coin::types::keys::KeyPair;
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "Coin Node")]
#[command(version = "0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    #[command(about = "Initialize new blockchain")]
    New {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        key: PathBuf,
    },
    #[command(about = "Start node")]
    Start {
        #[arg(short, long)]
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let success = match &cli.command {
        Commands::New { path, key } => new(path, key),
        Commands::Start { path } => start(path),
    };

    ExitCode::from(if success { 0 } else { 1 })
}

fn new(path: &PathBuf, key: &PathBuf) -> bool {
    println!(
        "Creating new chain at {} with key {}",
        path.display(),
        key.display()
    );
    let mut key_file = match File::open(key) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let key = match KeyPair::from_file(&mut key_file) {
        Ok(key) => key,
        Err(_) => return false,
    };

    let chain = SerializableChain::new(Chain::new(&key.public_key()));
    let mut chain_file = match File::create_new(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    match chain.to_file(&mut chain_file) {
        Ok(_) => println!("Chain saved to file: {}", path.display()),
        Err(_) => return false,
    };

    return true;
}

fn start(path: &PathBuf) -> bool {
    println!("Starting server with chain {}", path.display());
    let mut chain_file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let ser_chain = match SerializableChain::from_file(&mut chain_file) {
        Ok(chain) => chain,
        Err(_) => return false,
    };
    let chain = Chain::from_serializable(ser_chain);
    println!("{:#?}", chain);
    return true;
}
