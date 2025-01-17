use tokio::net::TcpListener;
use std::process::{ExitStatus, Stdio};
use std::process; 
use std::fs::{File};
use std::io::{self, Read, Write};
use std::error::Error;
use std::path::Path;
use std::fmt;
//use std::error::Error;

// A custom error type
#[derive(Debug)]
struct MyError {
    message: String,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

impl Error for MyError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port_path = Path::new("config/port");
    let matches = clap::App::new("B_Cli")
        .version("1.0")
        .author("Galen Sprout <galen.sprout@gmail.com>")
        .about("Start a BCoin node to handle account interactions")
	.subcommand(clap::SubCommand::with_name("start-node")
            .about("initialize a new node")
                    .help("Creates a node at an internally tracked port"))
        .subcommand(clap::SubCommand::with_name("create-account")
		    .about("create new account on chain")
		    .arg(clap::Arg::with_name("account-id")
			 .help("ID of user")
			 .required(true)
			 .validator(is_int_arg)
			 .index(1))
		    .arg(clap::Arg::with_name("starting-balance")
			 .help("Addresses starting balance")
			 .required(true)
			 .validator(is_float_arg)
			 .index(2)))
        .subcommand(clap::SubCommand::with_name("transfer")
		    .about("Transfer from A to B")
		    .arg(clap::Arg::with_name("from-address")
			 .help("Address to take from")
			 .validator(is_int_arg)
			 .required(true)
			 .index(1))
		    .arg(clap::Arg::with_name("to-address")
			 .help("Receiving Address")
			 .validator(is_int_arg)
			 .required(true)
			 .index(2))
		    .arg(clap::Arg::with_name("transfer-amount")
			 .help("Amount to transfer")
			 .validator(is_float_arg)
			 .required(true)
			 .index(3)))
        .subcommand(clap::SubCommand::with_name("balance")
		    .about("Return and show balance from given ID")
		    .arg(clap::Arg::with_name("account-id")
			 .help("Address to show balance of")
			 .validator(is_int_arg)
			 .required(true)
			 .index(1)))
        .get_matches();
    match matches.subcommand() {
	Some(("start-node",_)) => {
	    start_node(port_path).await;
	    Ok(())
	},
        Some(("create-account", sub_matches)) => {
	    let id = sub_matches.get_one::<String>("account-id").unwrap().parse::<i64>().unwrap();
            let balance = sub_matches.get_one::<String>("starting-balance").unwrap().parse::<f64>().unwrap();
	    let params = [("acct_id", id.to_string()), ("balance_0", balance.to_string())];
	    make_request(&port_path, "create-account", &params).await
	}
	Some (("transfer", sub_matches)) => {
	    let from = sub_matches.get_one::<String>("from-address").unwrap().parse::<i64>().unwrap();
	    let to = sub_matches.get_one::<String>("to-address").unwrap().parse::<i64>().unwrap();
	    let amount = sub_matches.get_one::<String>("transfer-amount").unwrap().parse::<f64>().unwrap();
	    let params =
		[("from_id", from.to_string())
		 , ("to_id", to.to_string())
		 , ("amount", amount.to_string())
		];
	    make_request(&port_path, "transfer", &params).await
	}
	Some (("balance", sub_matches)) => {
	    let acct_id = sub_matches.get_one::<String>("account-id").unwrap().parse::<i32>().unwrap();
	    let params = [("acct_id", acct_id.to_string())];
	    make_request(&port_path, "balance", &params).await
	},
        _ => {
            eprintln!("Invalid command or command not specified");
	    Ok(())
        }
    }
}

// Generic pattern for CLI -> Request 
async fn make_request(port_path: &Path, endpoint: &str, params: &[(&str, String)]) -> Result<(), Box<dyn std::error::Error>> {
    match read_port(&port_path) {
	Err(_) => {
	    println!("Couldn't read port path, this should never fail");
	    Err(Box::new(MyError { message: "Something went wrong".to_string() }))	
	},
	Ok(port) => {
	    let client = reqwest::Client::new();
	    //match client.get(&format!("http://localhost:{}/create-account", port)).query(&params).send().await 
	    match client.get(&format!("http://localhost:{}/{}", port, endpoint)).query(&params).send().await {
		Err(e) => {
		    //eprintln!(&format!("http://localhost:{}/{}", endpoint, port));
		    eprintln!("Failed to send request: {}", e);
		    Err(Box::new(MyError { message: "Failed to build request".to_string() }))	
		},
		Ok(response) => {
		    if !response.status().is_success() {
			println!("Status code unsuccessful");
			Err(Box::new(MyError { message: "Status code unsuccessful".to_string() }))	
		    } else {
			match response.text().await {
			    Err(e) => {
				eprintln!("Failed to read response body: {}", e);
				Err(Box::new(MyError { message: "Couldn't grab response text".to_string() }))	
			    }
			    Ok(body) => {
				println!("Response body: {}", body);
				Ok(())
			    }
			}
		    }
		}
	    }
	}
    }
}

// Start a node at an internally chosen and tracked port, in a new XTerminal
async fn start_node(port_config_file:&Path) {
    println!("Ensuring most up to date version of BChain node, please wait");
    match run_bash_wait("cd ../RustBlockchain && cargo update && nix-build") {
	Err(e) => {
	    println!("Process failure: {}", e);
	}
	Ok(status) => {
	    if !status.success() {
		println!("error building BChain Node, this is most likely because nix is not installed");
		println!("Retrying with system installed cargo");
		let _ = run_bash_wait("cd ../RustBlockchain && cargo update && cargo run");
		// TODO: retry with cargo build and then run exe
		   // AND warn that system dependencies may need to be installed
	    } else {
		match find_available_port().await {
		    Err(e) => eprintln!("Failed to find an available port: {}", e),
		    Ok(port) => {
			println!("Found available port: {}", port);
			run_in_new_terminal(&format!("../RustBlockchain/result/bin/rust_blockchain --port {}",port));
			let _ = write_port(port, port_config_file);
		    },
		    
		}
	    }
	    
	}
    }
    println!("started node")
}

fn run_in_new_terminal(command: &str) {
    let _ = process::Command::new("xterm")
        .arg("-e")
        .arg(format!("bash -c '{}; exec bash'", command))
        .spawn()
        .expect("Failed to start xterm with command");
}

fn run_bash_wait(cmd: &str) -> Result<ExitStatus, std::io::Error> {
    process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
	.stdout(Stdio::inherit())
	.stderr(Stdio::inherit())
        .status()
}

fn is_int_arg(v: &str) -> Result<(), String> {
    v.parse::<i32>()
     .map_err(|e| e.to_string())
     .map(|_| ())
}

fn is_float_arg(v: &str) -> Result<(), String> {
    v.parse::<f64>()
     .map_err(|e| e.to_string())
     .map(|_| ())
}

async fn find_available_port() -> Result<u32, std::io::Error> {
    // Bind to address 0.0.0.0:0 so the OS will assign a free port
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    Ok(addr.port() as u32) 
}

fn read_port(filename: &Path) -> io::Result<u32> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.trim().parse::<u32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse port"))
}

fn write_port(port: u32, filename: &Path) -> io::Result<()> {
    let mut file = File::create(filename)?;
    writeln!(file, "{}", port)
}
