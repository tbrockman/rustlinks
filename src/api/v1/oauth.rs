use std::collections::HashMap;

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state;

#[derive(Deserialize)]
struct LoginQuery {
    provider_url: String,
}

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
pub async fn login(
    state: web::Data<state::AppState>,
    query: web::Query<LoginQuery>,
) -> impl Responder {
    // Determine whether or not the server supports that issuer (exists an item in
    // map "issuer" -> "client_id")
    if let Some(provider) = state.oidc_providers.get(&query.provider_url) {
        // Otherwise, redirect to the provider's authorization endpoint (with
        // appropriately set params)

        HttpResponse::Ok().body("OK")
    } else {
        let urls = state.oidc_providers.keys().collect::<Vec<&String>>();
        HttpResponse::NotFound().body(format!(
            "Specified provider URL not found, currently have: {:?}",
            urls
        ))
    }
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
    HttpResponse::Ok().body("OK")
}
