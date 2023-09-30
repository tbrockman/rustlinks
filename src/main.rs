pub mod golink;
pub mod etcd;

use std::{collections::HashSet, sync::{Arc, Mutex}};

use actix_web::{get, web, App, HttpResponse, HttpServer, Either, Responder};
use etcd::Worker;
use etcd_rs::{Client, ClientConfig};

pub struct AppState {
    golinks: HashSet<golink::Golink>
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/{alias:.*}")]
async fn redirect(data: web::Data<AppState>, path: web::Path<String>) -> Either<web::Redirect, HttpResponse> {
    let alias = path.into_inner();
    println!("alias: {}", alias);
    let partial = golink::Golink {
        alias: alias.clone(),
        url: None
    };
    match data.golinks.get(&partial) {
        Some(golink) => Either::Left(web::Redirect::to(golink.url.clone().unwrap()).permanent()),
        None => Either::Right(HttpResponse::NotFound().finish())
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let mut golinks = HashSet::new();
    golinks.insert(golink::Golink {
        alias: "goog".to_string(),
        url: Some("https://google.com".to_string()),
    });
    golinks.insert(golink::Golink {
        alias: "gh".to_string(),
        url: Some("https://github.com".to_string()),
    });
    let state = web::Data::new(AppState{
        golinks: golinks
    });

    let cli = Client::connect(ClientConfig::new([
        "http://127.0.0.1:2379".into()
    ])).await.unwrap();

    let worker = Arc::new(Worker {
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
            result.unwrap();
        },
        result = server_result => {
            println!("server stopped");
            worker_stop.stop().await.unwrap();
            result.unwrap();
        }
    }

    println!("should be exiting...");
    Ok(())
}