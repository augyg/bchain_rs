use tokio::time::{self, Duration};
use std::convert::Infallible;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::sync::Arc;
use serde::Deserialize;
use std::net::SocketAddr;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Deserialize, Debug)]
struct CreateAccount {
    acct_id: i32,
    balance_0: f32
}

#[derive(Deserialize, Debug)]
struct Transfer {
    from_id: i32,
    to_id: i32,
    amount: f32
}

#[derive(Deserialize)]
struct ReadBalance {
    acct_id: i32,
}

#[derive(Debug)]
enum AccountAction {
    Action_CreateAccount(CreateAccount),
    Action_Transfer(Transfer),
}

#[derive(Debug)]
struct Accounts {
    accts: Vec<Account>
}

#[derive(Debug)]
struct Account {
    acct_id: i32,
    balance: f32
}


#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let matches = clap::App::new("BCoin Node")
        .version("1.0")
        .author("Galen Sprout <galen.sprout@gmail.com>")
        .about("Start a BCoin node to handle account interactions")
        .arg(clap::Arg::with_name("port")
             .short('p')
             .long("port")
             .takes_value(true)
             .help("Sets the port to run the server on"))
        .get_matches();
    let port = matches.value_of("port").unwrap_or("3000").parse::<u16>().expect("Invalid port number");
    let mut actions: Vec<AccountAction> = Vec::new();
    let actions_shared = Arc::new(Mutex::new(actions));
    let server_actions_shared = actions_shared.clone();
    let worker_actions_shared = actions_shared.clone();
	
    let mut accounts: Accounts = Accounts { accts: Vec :: new() };
    let accounts_shared = Arc::new(Mutex::new(accounts));
    let server_accounts_shared = accounts_shared.clone();
    let worker_accounts_shared = accounts_shared.clone();
    let _ = tokio::spawn(async move {
        println!("start server");
        if let Err(e) = my_server(port, server_actions_shared, server_accounts_shared).await {
            eprintln!("Server failed: {:?}", e);
        }
    });
    let periodic_task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
	    interval.tick().await; 
            println!("Processing Block");
	    let mut actions = worker_actions_shared.lock().await;
	    let mut accounts = worker_accounts_shared.lock().await;
	    println!("{:#?}", actions);
	    println!("{:#?}", accounts);
	    for action in actions.iter() {
		match action {
		    AccountAction::Action_CreateAccount(CreateAccount { acct_id, balance_0 }) => {
			// check if account exists if not: add it
			if accounts.accts.iter().any(|account| account.acct_id == *acct_id) {
			    println!("tried adding account ID which already exists");
			} else {
			    accounts.accts.push(Account { acct_id : *acct_id, balance : *balance_0 }); 
			}
		    },
		    AccountAction::Action_Transfer(Transfer { from_id, to_id, amount }) => {
			let from_index = accounts.accts.iter().position(|acc| acc.acct_id == *from_id);
			let to_index = accounts.accts.iter().position(|acc| acc.acct_id == *to_id);
			match (from_index, to_index) {
			    (Some(from_idx), Some(to_idx)) if from_idx != to_idx => {
				let from_balance = accounts.accts[from_idx].balance;
				// let to_balance = accounts.accts[to_idx].balance;
				
				if from_balance >= *amount {
				    accounts.accts[from_idx].balance -= amount;
				    accounts.accts[to_idx].balance += amount;
				    println!("Transferred {} BCoin from account {} to account {}", amount, from_id, to_id);
				} else {
				    println!("Not enough funds in account {}", from_id);
				}
			    },
			    _ => println!("One or both accounts not found, or same account referenced."),
			}
		    }
                }			
	    }
	    actions.clear();
	}
    });
    let _ = tokio::join!(periodic_task);
}


async fn my_server
    (port: u16
     , transaction_queue: Arc<Mutex<Vec<AccountAction>>>
     , accounts:Arc<Mutex<Accounts>>)
     -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening on http://{}", addr);
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn({
            let transaction_queue = transaction_queue.clone();  // Clone for the task scope
	    let accounts = accounts.clone();  // Clone for the task scope
            async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| {
                        // Clone for each request inside the service function
                        let queue_clone = transaction_queue.clone();
			let accounts_clone = accounts.clone();
                        handle_request(queue_clone, accounts_clone, req)
                    }))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}


//(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible>
async fn handle_request( transaction_queue:Arc<Mutex<Vec<AccountAction>>>
			 , accounts:Arc<Mutex<Accounts>>
			 , req: Request<hyper::body::Incoming>)
			-> Result<Response<Full<Bytes>>, Infallible>
{
    let path = req.uri().path();
    let method = req.method();

    match (method, path) {
        (&hyper::Method::GET, "/create-account") => {
	    let query_string = req.uri().query().unwrap();
	    match serde_urlencoded::from_str::<CreateAccount>(query_string) {
		Err(_) => {
		    Ok(Response::new(Full::new(Bytes::from("Invalid parameters"))))
		},
		Ok(create @ CreateAccount { acct_id, balance_0}) => {
                    {
                        let mut queue = transaction_queue.lock().await;
                        queue.push(AccountAction::Action_CreateAccount(create));
                    }		    
		    Ok(Response::new(Full::new(Bytes::from(
			format!("Pushed: Action_CreateAccount with ID: {} and starting balance: {}", acct_id, balance_0)
		    ))))
		},
	    }
        },
        (&hyper::Method::GET, "/transfer") => {
	    let query_string = req.uri().query().unwrap();
	    match serde_urlencoded::from_str::<Transfer>(query_string) {
		Err(_) => {
		    Ok(Response::new(Full::new(Bytes::from("Invalid parameters"))))
		},
		Ok(transfer @ Transfer { from_id:_, to_id:_, amount:_ }) => {
		    {
                        let mut queue = transaction_queue.lock().await;
                        queue.push(AccountAction::Action_Transfer(transfer));
                    }		    
		    Ok(Response::new(Full::new(Bytes::from(
			"Transfer pushed, call <url>/balance?acct_id=<desired account> in 10 seconds to see new balance"
		    ))))
		},
	    }
        },
	(&hyper::Method::GET, "/balance") => {
	    let query_string = req.uri().query().unwrap();
	    match serde_urlencoded::from_str::<ReadBalance>(query_string) {
		Err(_) => {
		    Ok(Response::new(Full::new(Bytes::from("Invalid parameters"))))
		},
		Ok(rb) => {
		    {
			let bchain_accounts = accounts.lock().await;
			match bchain_accounts.accts.iter().find(|&account| account.acct_id == rb.acct_id) {
			    Some(Account {acct_id:_, balance}) => {
				Ok(Response::new(Full::new(Bytes::from(
				    balance.to_string()
				))))
			    },
			    None => {
				Ok(Response::new(Full::new(Bytes::from(
				    "No account found for this id"
				))))
			    }
			}
		    }
		},
	    }
        },
        _ => {
	    let query = req.uri().query().unwrap();
	    println!("Query {}: failed, not recognized", query);
	    Ok(Response::new(Full::new(Bytes::from("Unrecognized request"))))
	}
    }
}
