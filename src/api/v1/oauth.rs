use std::collections::HashMap;

use actix_web::{
    get, post,
    web::{self, Redirect},
    Either, HttpResponse, Responder,
};
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

// #[get("/authorize")]
// pub async fn authorize(
//     state: web::Data<state::AppState>,
//     query: web::Query<LoginQuery>,
// ) -> Either<web::Redirect, HttpResponse> { // Determine whether or not the
//   server supports that issuer (exists an item in // map "issuer" ->
//   "client_id") if let Some(provider) =
//   state.oidc_providers.get(&query.provider_url) { // Otherwise, redirect to
//   the provider's authorization endpoint (with // appropriately set params)
//   let (url, csrf, nonce) = provider .authorize_url(
//   CoreAuthenticationFlow::Implicit(false), CsrfToken::new_random,
//   Nonce::new_random, ) .url();

//         Either::Left(Redirect::to(url.to_string()))
//     } else {
//         let urls = state.oidc_providers.keys().collect::<Vec<&String>>();
//         Either::Right(HttpResponse::NotFound().body(format!(
//             "Specified provider URL not found, currently have: {:?}",
//             urls
//         )))
//     }
// }

#[post("/logout")]
pub async fn logout() -> impl Responder {
    HttpResponse::Ok().body("OK")
}
