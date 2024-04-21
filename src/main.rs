#![feature(str_split_remainder)]
#![feature(let_chains)]
#![feature(async_closure)]
#![feature(const_trait_impl)]
#![feature(stmt_expr_attributes)]

pub mod api;
pub mod cli;
pub mod errors;
pub mod oidc;
pub mod redirect;
pub mod rustlink;
pub mod state;
pub mod storage;
pub mod tls;
pub mod ui;
pub mod util;
pub mod worker;
#[cfg(feature = "ui")]
use std::fs::read_to_string;
use std::{
    fs::{File, OpenOptions},
    sync::Arc,
};

#[cfg(feature = "oauth")]
use actix_files::Files;
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use actix_web_opentelemetry::RequestMetrics;
use actix_web_opentelemetry::RequestTracing;
use errors::RustlinksError;
use etcd_rs::{Client, ClientConfig, Endpoint};
use heed::EnvOpenOptions;
use opentelemetry::global;
use opentelemetry_sdk::runtime::TokioCurrentThread;
use tokio::sync::{Mutex, RwLock};
#[cfg(feature = "oauth")]
use url::Url;
use worker::Worker;

type RustlinkAlias = String;

const DB_NAME: &str = "links.db";

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
            .split(',')
            .map(|s| s.into())
            .collect::<Vec<Endpoint>>(),
    ))
    .await?;

    let cli::Commands::Start {
        hostname,
        port,
        db_path,
        db_map_size,
        cert_file,
        key_file,
    }: cli::Commands = cli.command
    else {
        unreachable!();
    };

    std::fs::create_dir_all(&db_path)?;

    let env = EnvOpenOptions::new().map_size(db_map_size).open(&db_path)?;

    #[cfg(feature = "oauth")]
    let oidc_providers = oidc::provider::populate_provider_metadata(oidc_providers).await;

    let state = web::Data::new(state::AppState {
        etcd_client: Arc::new(etcd_client),
        rustlink_store: Arc::new(storage::LMDB::new(env)),
        read_only: cli.global.read_only,
        #[cfg(feature = "oauth")]
        oauth_redirect_endpoint: oauth_redirect_endpoint.clone(),
        #[cfg(feature = "oauth")]
        oidc_providers: Arc::new(RwLock::new(oidc_providers)),
        #[cfg(feature = "oauth")]
        login_path: login_path.clone(),
        #[cfg(feature = "ui")]
        js_source: Arc::new(RwLock::new(read_to_string("./src/ui/dist/index.js")?)),
    });
    let worker = Box::new(Worker {
        state: state.clone(),
        cancel: Arc::new(Mutex::new(None)),
        sleep: Arc::new(Mutex::new(None)),
    });

    #[cfg(feature = "oauth")]
    let url = match Url::parse(oauth_redirect_endpoint.as_str()) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Failed to parse OAuth redirect endpoint: {:?}", e);
            return Err(RustlinksError::OAuthEndpointParseError(e));
        }
    };

    let server = HttpServer::new(move || {
        let mut api = web::scope("/api/v1")
            .service(web::scope("/health").service(api::v1::health::check))
            .service(
                // TODO: parse bearer auth middleware
                web::scope("/links")
                    .service(api::v1::links::create_rustlink)
                    .service(api::v1::links::delete_rustlink)
                    .service(api::v1::links::get_rustlinks),
            );

        #[cfg(feature = "oauth")]
        {
            api = api.service(web::scope("/oauth"));
        }

        let mut app = App::new()
            .app_data(Data::clone(&state))
            .service(api)
            .service(redirect::redirect)
            .wrap(RequestMetrics::default())
            .wrap(RequestTracing::new());

        #[cfg(feature = "oauth")]
        {
            app = app
                .service(web::resource(url.path()).route(web::get().to(api::v1::oauth::callback)))
                .service(web::scope(login_path.as_str()).service(ui::route::index));
        }

        #[cfg(feature = "ui")]
        {
            app = app
                .service(web::scope("/").service(ui::route::index))
                .service(Files::new("/_ui/styles", "./src/ui/dist/styles").show_files_listing())
                .service(Files::new("/_ui/images", "./src/ui/dist/images").show_files_listing())
                .service(Files::new("/_ui/scripts", "./src/ui/dist/scripts").show_files_listing())
        }
        return app;
    });

    let server_future: Server;

    if let Some(cert) = cert_file
        && let Some(key) = key_file
    {
        let config = tls::load_rustls_config(cert, key)?;
        server_future = server.bind_rustls_0_22((hostname, port), config)?.run();
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
