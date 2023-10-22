use std::collections::HashMap;

use actix_web::{web, HttpResponse, Responder};

pub async fn callback(query: web::Query<HashMap<String, String>>) -> impl Responder {
    HttpResponse::Ok().body("OK")
}
