use actix_web::{get, HttpResponse, Responder};

#[get("/")]
pub async fn check() -> impl Responder {
    // TODO: return etcd connection status
    HttpResponse::Ok().body("OK")
}
