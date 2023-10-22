use actix_web::{get, HttpResponse, Responder};

#[get("/health")]
pub async fn check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}
