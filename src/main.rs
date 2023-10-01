#![feature(str_split_remainder)]
#![feature(let_chains)]

pub mod golink;
pub mod datastore;
pub mod util;

use std::{collections::HashMap, sync::{Arc, Mutex, RwLock}};

use actix_web::{get, web, App, HttpResponse, HttpServer, Either, Responder};
use datastore::Worker;
use etcd_rs::{Client, ClientConfig};

type GolinkAlias = String;
pub struct AppState {
    golinks: Arc<RwLock<HashMap<GolinkAlias, golink::Golink>>>
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/{alias:.*}")]
async fn redirect(data: web::Data<AppState>, path: web::Path<String>) -> Either<web::Redirect, HttpResponse> {
    let full = path.into_inner();
    let mut split = full.split(" ");
    let alias = split.next().unwrap();
    let params = split.remainder();

    println!("alias: {}", alias);
    println!("params: {:?}", params);
    println!("have golinks: {:?}", data.golinks.read().unwrap());
    let guard = data.golinks.read().unwrap();
    match guard.get(alias) {
        Some(golink) => Either::Left(web::Redirect::to(golink.url.clone().unwrap()).permanent()),
        None => Either::Right(HttpResponse::NotFound().finish())
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let mut golinks: HashMap<GolinkAlias, golink::Golink> = HashMap::new();
    golinks.insert("goog".to_string(), golink::Golink {
        alias: "goog".to_string(),
        url: Some("https://google.com".to_string()),
    });
    golinks.insert("gh".to_string(), golink::Golink {
        alias: "gh".to_string(),
        url: Some("https://github.com".to_string()),
    });
    let state = web::Data::new(AppState{
        golinks: Arc::new(RwLock::new(golinks))
    });

    let cli = Client::connect(ClientConfig::new([
        "http://127.0.0.1:2379".into()
    ])).await.unwrap();

    let worker = Box::new(Worker {
        state: state.clone(),
        client: cli,
        cancel: Arc::new(Mutex::new(None))
    });
    let server_future = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(web::scope("/api")
                .service(health))
            .service(redirect)
    })
    .bind(("127.0.0.1", 8080))?.run();
    let worker_start = worker.clone();
    let worker_stop = worker.clone();

    let server_result = tokio::spawn(server_future);
    let etcd_result = tokio::spawn(async move {
        worker_start.start().await
    });

    tokio::select! {
        result = etcd_result => {
            println!("etcd worker stopped");
            // TODO: handle error and attempt to recover
            // application can continue to function without etcd
            result.unwrap();
        },
        _ = server_result => {
            println!("server stopped");
            worker_stop.stop().await.unwrap();
        }
    }

    println!("should be exiting...");
    Ok(())
}