use std::collections::HashMap;

use actix_web::{get, post, web, HttpResponse, Responder};

// TODO:
// either:
// store map of `kid` => `jwks` in AppState
// Add middleware(/guard?) to validate JWTs based on retrieved keys
// or:
// always verify JWTs remotely and accept network request cost, but allow
// instant revocation

pub async fn callback(query: web::Query<HashMap<String, String>>) -> impl Responder {
    // TODO:
    HttpResponse::Ok().body("OK")
}

#[get("/login")]
pub async fn login() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
    HttpResponse::Ok().body("OK")
}
