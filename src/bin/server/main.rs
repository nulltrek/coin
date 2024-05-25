use clap::{Parser, Subcommand};
use coin::chain::{Chain, SerializableChain};
use coin::traits::io::{FileIO, JsonIO};
use coin::types::keys::KeyPair;
use rouille::{router, Response, ResponseBody, Server};
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

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

trait CommonResponses {
    fn ok(json: &str) -> Response {
        Response {
            status_code: 200,
            headers: vec![(
                "Content-Type".into(),
                "application/json; charset=utf-8".into(),
            )],
            data: ResponseBody::from_string(json),
            upgrade: None,
        }
    }

    fn server_error() -> Response {
        Response {
            status_code: 500,
            headers: vec![(
                "Content-Type".into(),
                "application/json; charset=utf-8".into(),
            )],
            data: ResponseBody::from_string(""),
            upgrade: None,
        }
    }

    fn not_found() -> Response {
        Response {
            status_code: 404,
            headers: vec![(
                "Content-Type".into(),
                "application/json; charset=utf-8".into(),
            )],
            data: ResponseBody::from_string(""),
            upgrade: None,
        }
    }
}

impl CommonResponses for Response {}

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
    let chain = Arc::new(Mutex::new(Chain::from_serializable(ser_chain)));
    if !chain.lock().unwrap().validate_chain() {
        println!("Blockchain validation failed!");
        return false;
    }

    let chain_ref = chain.clone();
    let server = Server::new("127.0.0.1:8080", move |request| {
        router!(request,
        (GET) (/chain) => {
            match SerializableChain::new(chain_ref.lock().unwrap().clone()).to_json() {
                Ok(chain) => Response::ok(&chain),
                Err(_) => Response::server_error(),
            }
        },
        _ => Response::not_found())
    })
    .unwrap();

    println!("Listening on {:?}", server.server_addr());
    let (handle, sender) = server.stoppable();

    ctrlc::set_handler(move || {
        println!("CTRL+C");
        sender.send(()).unwrap();
    })
    .expect("Error setting SIGTERM handler");

    handle.join().unwrap();
    return true;
}
