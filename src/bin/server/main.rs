use clap::{Parser, Subcommand};
use coin::chain::{Chain, SerializableChain};
use coin::consensus::{ConsensusRules, Halving, Target};
use coin::core::keys::{KeyPair, PublicKey};
use coin::core::transaction::Transaction;
use coin::mining::miner::Miner;
use coin::traits::io::{FileIO, JsonIO};
use coin::utils::utxos_to_json;
use rouille::{router, Response, ResponseBody, Server};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
        #[arg(short, long)]
        recipient: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let success = match &cli.command {
        Commands::New { path, key } => command_new(path, key),
        Commands::Start { path, recipient } => command_start(path, recipient),
    };

    ExitCode::from(if success { 0 } else { 1 })
}

fn command_new(path: &PathBuf, key: &PathBuf) -> bool {
    println!(
        "Creating new chain at {} with key {}",
        path.display(),
        key.display()
    );
    let key = match KeyPair::from_file(&key) {
        Ok(key) => key,
        Err(_) => {
            println!("Failed to read key from file!");
            return false;
        }
    };

    let chain = Chain::new_with_consensus(
        &key.public_key(),
        ConsensusRules::new(Target::from_leading_zeros(15), 10000, Halving::Inf),
    );
    match SerializableChain::new(chain).to_file(path) {
        Ok(_) => println!("Chain saved to file: {}", path.display()),
        Err(_) => {
            println!("Failed to save chain to file!");
            return false;
        }
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

    fn client_error() -> Response {
        Response {
            status_code: 400,
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

enum MinerCommand {
    Stop,
    Mine,
}

fn command_start(path: &PathBuf, recipient: &PathBuf) -> bool {
    println!("Starting server with chain {}", path.display());

    // SETUP BLOCKCHAIN
    let chain = match SerializableChain::from_file(path) {
        Ok(chain) => Chain::from_serializable(chain),
        Err(_) => {
            println!("Cannot deserialize blockchain!");
            return false;
        }
    };

    let chain = Arc::new(Mutex::new(chain));
    if !chain.lock().unwrap().validate_chain() {
        println!("Blockchain validation failed!");
        return false;
    }

    // SETUP RECIPIENT KEY
    let key = match KeyPair::from_file(recipient) {
        Ok(key) => key,
        Err(_) => {
            println!("Failed to read key from file!");
            return false;
        }
    };

    // SETUP MINER
    let miner = Arc::new(Mutex::new(Miner::new(key.public_key())));

    let chain_miner_ref = chain.clone();
    let miner_miner_ref = miner.clone();
    let (miner_sender, miner_receiver) = mpsc::channel();
    let miner_task = thread::spawn(move || {
        let mine = || {
            let mut chain = chain_miner_ref.lock().unwrap();
            match miner_miner_ref.lock().unwrap().mine(&chain) {
                Ok(block) => {
                    println!("Trying to add block: {:#?}", block);
                    match chain.add_block(block) {
                        Ok(height) => {
                            println!("Mining successful, inserted block with height: {}", height)
                        }
                        Err(_) => println!("Mining failed, block is not valid."),
                    }
                },
                Err(err) => {
                    println!("Mining aborted: {:?}", err);
                }
            }
        };

        loop {
            match miner_receiver.recv_timeout(Duration::from_secs(60)) {
                Ok(command) => match command {
                    MinerCommand::Stop => return,
                    MinerCommand::Mine => {
                        if miner_miner_ref.lock().unwrap().pool.len() > 10 {
                            mine();
                        }
                    }
                },
                Err(_) => mine(),
            }
        }
    });

    // SETUP WEBSERVER
    let chain_ref = chain.clone();
    let miner_ref = miner.clone();
    let miner_sender_ref = miner_sender.clone();
    let server_task = Server::new("127.0.0.1:8080", move |request| {
        router!(request,
        (GET) (/chain) => {
            println!("GET /chain");
            match SerializableChain::new(chain_ref.lock().unwrap().clone()).to_json() {
                Ok(chain) => Response::ok(&chain),
                Err(_) => Response::server_error(),
            }
        },
        (POST) (/chain) => {
            println!("POST /chain");
            let mut body = match request.data() {
                None => return Response::client_error(),
                Some(body) => body
            };

            let mut buf = Vec::new();
            if body.read_to_end(&mut buf).is_err() {
                return Response::server_error();
            }

            let tx = match Transaction::from_json(String::from_utf8(buf).unwrap().as_str()) {
                Ok(tx) => tx,
                Err(_) => return Response::client_error(),
            };

            let mut miner = miner_ref.lock().unwrap();
            if miner.add_tx(&chain_ref.lock().unwrap(), tx) {
                let _ = miner_sender_ref.send(MinerCommand::Mine);
                Response::ok("")
            } else {
                Response::client_error()
            }
        },
        (GET) (/utxos/all) => {
            println!("GET /utxos/all");
            let utxos = chain_ref.lock().unwrap().find_all_utxos();
            Response::ok(&utxos_to_json(&utxos).unwrap().as_str())
        },
        (GET) (/utxos/{addr: String}) => {
            println!("GET /utxos");
            let pubkey = match PublicKey::from_hex_str(addr.as_str()) {
                Ok(key) => key,
                Err(_) => return Response::client_error(),
            };
            let utxos = chain_ref.lock().unwrap().find_utxos_for_key(&pubkey);
            Response::ok(&utxos_to_json(&utxos).unwrap().as_str())
        },
        (GET) (/pool) => {
            println!("GET /pool");
            let transactions: Vec<Transaction> = miner_ref.lock().unwrap().pool.values().map(|tx| tx.clone()).collect();
            Response::ok(&serde_json::to_string(&transactions).unwrap())
        },
        _ => {
            println!("{:?}", request);
            Response::not_found()
        }
        )})
    .unwrap();

    println!("Listening on {:?}", server_task.server_addr());
    let (server_task, server_sender) = server_task.stoppable();

    // SETUP HANDLERS AND TEARDOWN
    ctrlc::set_handler(move || {
        println!("CTRL+C");
        miner_sender.send(MinerCommand::Stop).unwrap();
        server_sender.send(()).unwrap();
    })
    .expect("Error setting SIGTERM handler");

    miner_task.join().unwrap();
    server_task.join().unwrap();

    let chain = Arc::try_unwrap(chain).unwrap().into_inner().unwrap();
    match SerializableChain::new(chain).to_file(path) {
        Ok(_) => println!("Chain saved to file: {}", path.display()),
        Err(_) => {
            println!("Failed to save chain to file!");
            return false;
        }
    };

    return true;
}
