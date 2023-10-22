#![feature(str_split_remainder)]
#![feature(let_chains)]
#![feature(async_closure)]

pub mod api;
pub mod cli;
pub mod errors;
pub mod redirect;
pub mod rustlink;
pub mod state;
pub mod tls;
pub mod util;
pub mod worker;

use std::{
    fs::{File, OpenOptions},
    sync::Arc,
};

use actix_web::{dev::Server, web, App, HttpServer};
use actix_web_opentelemetry::RequestMetrics;
use actix_web_opentelemetry::RequestTracing;
use errors::RustlinksError;
use etcd_rs::{Client, ClientConfig, Endpoint};
use opentelemetry::{global, runtime::TokioCurrentThread};
use tokio::sync::{Mutex, RwLock};
use worker::Worker;

type RustlinkAlias = String;

const LINK_FILENAME: &str = "links.json";

async fn start(cli: cli::RustlinksOpts) -> Result<(), errors::RustlinksError> {
    // Enable tracing
    // TODO: make configurable

    let _ = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(TokioCurrentThread)?;

    // Enable metrics
    // TODO: make configurable

    let _ = opentelemetry_otlp::new_pipeline()
        .metrics(TokioCurrentThread)
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .build()?;

    // TODO: handle connection error here without panic'ing?
    let etcd_client = Client::connect(ClientConfig::new(
        cli.global
            .etcd_endpoints
            .unwrap()
            .split(',')
            .map(|s| s.into())
            .collect::<Vec<Endpoint>>(),
    ))
    .await
    .unwrap();

    let cli::Commands::Start {
        hostname,
        port,
        data_dir,
        cert,
        key,
        oidc_well_known_config_url,
        oidc_client_id,
        oauth_redirect_endpoint,
    }: cli::Commands = cli.command
    else {
        unreachable!();
    };

    let links_filepath = data_dir.join(LINK_FILENAME);

    match links_filepath.parent() {
        Some(parent) => {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        _ => {}
    }
    let links_file: Option<File> = match OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(links_filepath.clone())
    {
        Ok(f) => Some(f),
        Err(n) => {
            eprint!(
                "Error opening/creating links file at [{:?}]: {:?}",
                links_filepath, n
            );
            None
        }
    };
    let state = web::Data::new(state::AppState {
        rustlinks: Arc::new(RwLock::new(Default::default())),
        etcd_client: Arc::new(etcd_client),
        revision: Arc::new(RwLock::new(0)),
        links_file: Arc::new(RwLock::new(links_file)),
        read_only: cli.global.read_only,
        oauth_redirect_endpoint: Arc::new(oauth_redirect_endpoint.clone()),
    });
    let worker = Box::new(Worker {
        state: state.clone(),
        cancel: Arc::new(Mutex::new(None)),
        sleep: Arc::new(Mutex::new(None)),
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(
                web::scope("/api/v1/")
                    .service(web::scope("/health").service(api::v1::health::check))
                    .service(
                        web::scope("/links")
                            .service(api::v1::links::create_rustlink)
                            .service(api::v1::links::delete_rustlink)
                            .service(api::v1::links::get_rustlinks),
                    )
                    .service(
                        web::scope("/oauth").service(
                            web::resource(oauth_redirect_endpoint.as_str())
                                .route(web::get().to(api::v1::oauth::callback)),
                        ),
                    ),
            )
            .service(redirect::redirect)
            .wrap(RequestMetrics::default())
            .wrap(RequestTracing::new())
    });

    let server_future: Server;

    if let Some(cert) = cert && let Some(key) = key {
        let config = tls::load_rustls_config(cert, key)?;
        server_future = server.bind_rustls_021((hostname, port), config)?.run();
    } else {
        server_future = server.bind((hostname, port))?.run();
    }

    let worker_start = worker.clone();
    let worker_stop = worker.clone();

    let server_result = tokio::spawn(server_future);
    let etcd_result = tokio::spawn(async move { worker_start.start().await });

    let exit_result = tokio::select! {
        _ = etcd_result => {
            println!("etcd worker stopped");
            // TODO: handle error and attempt to recover
            // application can continue to function without etcd
            Ok(())
        },
        _ = server_result => {
            println!("server stopped");
            worker_stop.stop().await
        }
    };

    global::shutdown_tracer_provider();
    exit_result
}
async fn install(cli: cli::RustlinksOpts) -> Result<(), RustlinksError> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), errors::RustlinksError> {
    let cli = cli::RustlinksOpts::parse();

    match cli.command {
        cli::Commands::Start { .. } => start(cli).await,
        cli::Commands::Install { .. } => install(cli).await,
    }
}
