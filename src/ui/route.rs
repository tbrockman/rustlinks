use actix_web::{
    get,
    http::StatusCode,
    web::{self, Bytes},
    Error, HttpRequest, HttpResponse, Responder,
};
use ssr_rs::Ssr;

use crate::state::AppState;

#[get("*")]
pub async fn index(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let oidc_providers: Vec<crate::oidc::provider::OIDCProvider> =
        data.oidc_providers.read().await.clone();
    let oidc_providers_string = serde_json::to_string(&oidc_providers).unwrap_or("[]".to_string());

    let props = format!(
        r##"{{
            "location": "{}",
            "context": {{}},
            "oidc_providers": {},
            "oauth_redirect_endpoint": "{}",
            "login_path": "{}"
        }}"##,
        req.uri(),
        oidc_providers_string,
        data.oauth_redirect_endpoint,
        data.login_path,
    );

    let source = data.js_source.read().await;
    let js: Ssr<'_> = ssr_rs::Ssr::new(source.to_string(), "SSR"); // TODO: figure out how to debug SSR errors better

    let response_body = js.render_to_string(Some(props.as_str()));
    let bytes = Bytes::from(response_body);
    let body: tokio_stream::Once<Result<Bytes, Error>> = tokio_stream::once(Ok(bytes));

    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .streaming(body)
}
