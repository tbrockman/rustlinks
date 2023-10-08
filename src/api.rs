use actix_web::{delete, get, put, web, HttpResponse, Responder};
use etcd_rs::KeyValueOp;

use crate::{rustlink::Rustlink, state::AppState};

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/links")]
pub async fn get_rustlinks(data: web::Data<AppState>) -> impl Responder {
    let rustlinks = data.rustlinks.read().await;
    return HttpResponse::Ok().json(rustlinks.values().collect::<Vec<&Rustlink>>());
}

#[put("/links/{alias}")]
pub async fn create_rustlink(
    data: web::Data<AppState>,
    path: web::Path<String>,
    rustlink: web::Json<Rustlink>,
) -> impl Responder {
    let alias = path.into_inner();
    if let Ok(bytes) = serde_json::to_vec(&rustlink) {
        match data.client.put((alias, bytes)).await {
            Ok(_) => return HttpResponse::Ok().body("OK"),
            Err(e) => {
                eprintln!("Failed to PUT to etcd: {:?}", e);
                return HttpResponse::InternalServerError().body("Internal Server Error");
            }
        }
    } else {
        HttpResponse::BadRequest().body(format!("Failed to parse JSON: {:?}", rustlink))
    }
}

#[delete("/links/{alias}")]
pub async fn delete_rustlink(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let alias = path.into_inner();
    match data.client.delete(alias).await {
        Ok(_) => return HttpResponse::Ok().body("OK"),
        Err(e) => {
            eprintln!("Failed to DELETE from etcd: {:?}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    }
}
