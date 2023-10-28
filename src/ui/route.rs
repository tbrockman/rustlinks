use actix_web::{
    get,
    http::StatusCode,
    web::{self, Bytes},
    Error, HttpRequest, HttpResponse, Responder,
};
use ssr_rs::Ssr;

use crate::state::AppState;

#[get("*")]
async fn index(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let props = format!(
        r##"{{
            "location": "{}",
            "context": {{}}
        }}"##,
        req.uri()
    );

    let source = data.js_source.read().await;
    let js: Ssr<'_> = ssr_rs::Ssr::new(source.to_string(), "SSR");

    let response_body = js.render_to_string(None);
    let bytes = Bytes::from(response_body);
    let body: tokio_stream::Once<Result<Bytes, Error>> = tokio_stream::once(Ok(bytes));

    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .streaming(body)
}
