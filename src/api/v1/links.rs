use actix_web::{delete, get, put, web, HttpResponse, Responder};
use etcd_rs::{KeyValueOp, PutRequest};

use crate::{rustlink::Rustlink, state::AppState, util};

#[get("/")]
pub async fn get_rustlinks(data: web::Data<AppState>) -> impl Responder {
    // TODO: cursor-based pagination
    // TODO: search queries
    let rustlinks = data.rustlinks.read().await;
    return HttpResponse::Ok().json(rustlinks.values().collect::<Vec<&Rustlink>>());
}

#[put("/{alias}")]
pub async fn create_rustlink(
    data: web::Data<AppState>,
    path: web::Path<String>,
    rustlink: web::Json<Rustlink>,
) -> impl Responder {
    println!("creating rust link");
    if let Ok(bytes) = serde_json::to_vec(&rustlink) {
        let key = util::alias_to_key(&path.into_inner());
        println!("using key: {:?} and value: {:?}", key, rustlink);
        let req = PutRequest::new(key, bytes.to_owned());
        match data.etcd_client.put(req).await {
            Ok(_) => HttpResponse::Ok().body("OK"),
            Err(e) => {
                eprintln!("Failed to PUT to etcd: {:?}", e);
                return HttpResponse::InternalServerError().body("Internal Server Error");
            }
        }
    } else {
        HttpResponse::BadRequest().body(format!("Failed to parse JSON: {:?}", rustlink))
    }
}

#[delete("/{alias}")]
pub async fn delete_rustlink(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let alias = path.into_inner();
    match data.etcd_client.delete(alias).await {
        Ok(_) => return HttpResponse::Ok().body("OK"),
        Err(e) => {
            eprintln!("Failed to DELETE from etcd: {:?}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    }
}
