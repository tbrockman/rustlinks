#![feature(str_split_remainder)]
#![feature(let_chains)]
#![feature(async_closure)]

pub mod api;
pub mod datastore;
pub mod rustlink;
pub mod util;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc, fs::{File, OpenOptions},
};

use actix_web::{get, web, App, Either, HttpResponse, HttpServer};
use clap::{Parser, Subcommand};
use datastore::Worker;
use etcd_rs::{Client, ClientConfig, Endpoint};
use tokio::sync::{RwLock, Mutex};

use crate::errors::StartError;

type RustlinkAlias = String;

pub mod errors;
pub mod state;

const LINK_FILENAME: &str = "links.json";

#[get("/{alias:.*}")]
async fn redirect(
    data: web::Data<state::AppState>,
    path: web::Path<String>,
) -> Either<web::Redirect, HttpResponse> {
    let full = path.into_inner();
    let mut split = full.split(' ');
    let alias = split.next().unwrap();
    let params = split.remainder();

    println!("alias: {}", alias);
    println!("params: {:?}", params);
    println!("have links: {:?}", data.rustlinks.read().await);
    let guard = data.rustlinks.read().await;
    match guard.get(alias) {
        Some(rustlink) => {
            Either::Left(web::Redirect::to(rustlink.url.clone().unwrap()).permanent())
        }
        None => Either::Right(HttpResponse::NotFound().finish()),
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional path to a .toml config file (precendence = args > env vars >
    /// config file)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Turn debug logging on
    #[arg(short, long)]
    debug: bool,
    // TODO: tracing backend
    // #[arg(short, long)]
    // telemetry: bool,
    /// Hostname(s) or IP address(es) of the etcd server(s)
    #[arg(
        long,
        use_value_delimiter = true,
        default_value = "http://127.0.0.1:2379"
    )]
    etcd_endpoints: Option<String>,

    /// Path to CA certificate to be used for communication with etcd (if
    /// passed, TLS will be used)
    #[arg(long)]
    etcd_ca_cert: Option<PathBuf>,

    /// Username to use for etcd authentication
    #[arg(long, default_value = "rustlinks")]
    etcd_username: Option<String>,

    /// Password to use for etcd authentication
    #[arg(long, default_value = "admin")]
    etcd_password: Option<String>,

    /// Flag to indicate whether server should run as primary (clients are
    /// secondary)
    #[arg(long, default_value_t = true)]
    primary: bool,

    /// Certificate file to be used by the server for TLS
    #[arg(long)]
    cert_file: Option<PathBuf>,

    /// Key file to be used by the server for TLS
    #[arg(long)]
    key_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the server
    Start {
        /// Hostname or IP address to bind to
        #[arg(long, default_value = "127.0.0.1")]
        hostname: String,

        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Path to a directory to persist Rustlink data
        #[arg(long, default_value = ".rustlinks/")]
        data_dir: PathBuf,
    },
    /// Configure the application, automatically performs certificate
    /// generation, role provisioning, and other setup required for
    /// the application to run successfully
    Configure {},
}

async fn start(cli: Cli) -> Result<(), errors::StartError> {
    let mut rustlinks: HashMap<RustlinkAlias, rustlink::Rustlink> = HashMap::new();
    rustlinks.insert(
        "goog".to_string(),
        rustlink::Rustlink {
            alias: "goog".to_string(),
            url: Some("https://google.com".to_string()),
        },
    );
    rustlinks.insert(
        "gh".to_string(),
        rustlink::Rustlink {
            alias: "gh".to_string(),
            url: Some("https://github.com".to_string()),
        },
    );
    // TODO: handle connection error here without panic'ing?
    let client = Client::connect(ClientConfig::new(
        cli.etcd_endpoints
            .unwrap()
            .split(',')
            .map(|s| s.into())
            .collect::<Vec<Endpoint>>(),
    ))
    .await
    .unwrap();

    let Commands::Start{hostname, port, data_dir}: Commands = cli.command else {
        unreachable!();   
    };
    let links_filepath = data_dir.join(LINK_FILENAME);
    // TODO: create necessary dirs recursively
    let links_file: Option<File> = match OpenOptions::new().write(true).create(true).append(false).open(links_filepath.clone()) {
        Ok(f) => Some(f),
        Err(n) => {
            eprint!("Error creating links file: {:?} at location: {:?}", n, links_filepath);
            None
        }
    };

    let state = web::Data::new(state::AppState {
        rustlinks: Arc::new(RwLock::new(rustlinks)),
        client: Arc::new(client),
        revision: Arc::new(RwLock::new(0)),
        links_file: Arc::new(RwLock::new(links_file)),
    });
    let worker = Box::new(Worker {
        state: state.clone(),
        cancel: Arc::new(Mutex::new(None)),
        sleep: Arc::new(Mutex::new(None))
    });
    let server_future = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(
                web::scope("/api")
                    .service(api::health)
                    .service(api::create_rustlink)
                    .service(api::delete_rustlink)
                    .service(api::get_rustlinks),
            )
            .service(redirect)
    })
    .bind(("127.0.0.1", 8080))?
    .run();
    let worker_start = worker.clone();
    let worker_stop = worker.clone();

    let server_result = tokio::spawn(server_future);
    let etcd_result = tokio::spawn(async move { worker_start.start().await });

    tokio::select! {
        _ = etcd_result => {
            println!("etcd worker stopped");
            // TODO: handle error and attempt to recover
            // application can continue to function without etcd
            Ok(())
        },
        _ = server_result => {
            println!("server stopped");
            match worker_stop.stop().await {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("failed to stop worker: {:?}", e);
                    Err(StartError::EtcdError(e))
                }
            }
        }
    }
}
async fn configure(cli: Cli) -> Result<(), ()> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), errors::StartError> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Start { .. } => start(cli).await,
        _ => todo!(),
    };
    result
}
