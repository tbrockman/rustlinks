use actix_web::{get, web, Either, HttpResponse};
use opentelemetry::{
    global,
    trace::{get_active_span, Tracer},
};

use crate::state;

#[get("/{alias:.*}")]
pub async fn redirect(
    state: web::Data<state::AppState>,
    path: web::Path<String>,
) -> Either<web::Redirect, HttpResponse> {
    let tracer = global::tracer("redirect");
    tracer
        .in_span("render-url-template-and-redirect", async move |_| {
            let full = path.into_inner();
            let mut split = full.split(" ");
            let alias = split.next().unwrap();
            let params: Vec<&str> = split.collect();
            let rustlinks = state.rustlinks.read().await;

            get_active_span(|span| match rustlinks.get(alias) {
                Some(rustlink) => {
                    if let Ok(url) = rustlink.render(params.clone()) {
                        // Increment counter for this alias
                        let meter = global::meter("");
                        let builder = meter.u64_counter("rustlinks.redirects");
                        let counter = builder.init();
                        counter.add(
                            1,
                            [opentelemetry::KeyValue::new(
                                "rustlinks.alias",
                                alias.to_string(),
                            )]
                            .as_ref(),
                        );
                        // Attach alias metadata to span
                        span.set_attribute(opentelemetry::KeyValue::new(
                            "rustlinks.alias",
                            alias.to_string(),
                        ));
                        span.set_attribute(opentelemetry::KeyValue::new(
                            "rustlinks.url",
                            url.clone(),
                        ));
                        span.set_attribute(opentelemetry::KeyValue::new(
                            "rustlinks.params",
                            params.join(" ").to_string(),
                        ));
                        Either::Left(web::Redirect::to(url).permanent())
                    } else {
                        Either::Right(HttpResponse::InternalServerError().finish())
                    }
                }
                None => Either::Right(HttpResponse::NotFound().finish()),
            })
        })
        .await
}

#[cfg(test)]
mod integration_tests {
    use std::{collections::HashMap, sync::Arc};

    use actix_web::{test, App};
    use etcd_rs::{Client, ClientConfig, Endpoint};
    use tokio::sync::RwLock;

    use super::*;
    use crate::{rustlink::Rustlink, state::AppState, RustlinkAlias};

    #[actix_web::test]
    async fn it_templates_no_items_with_no_format_string() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q=abcdefg".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q=abcdefg"
        );
    }

    #[actix_web::test]
    async fn it_templates_no_items_with_format_string() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q={}".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q="
        );
    }

    #[actix_web::test]
    async fn it_templates_no_items_with_no_format_string_but_has_params() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q=abcdefg".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test%20testparameter").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q=abcdefg"
        );
    }

    #[actix_web::test]
    async fn it_templates_items_with_format_string_and_params() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q={}".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test%20testparameter").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q=testparameter"
        );
    }

    #[actix_web::test]
    async fn it_templates_items_with_format_string_and_params_with_spaces() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q={}".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test%20multiple%20spaces%20test").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q=multiple%20spaces%20test"
        );
    }

    #[actix_web::test]
    async fn it_templates_multiple_input_parameters() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
        rustlinks.insert(
            "test".to_string(),
            Rustlink {
                url: "https://google.com/search?q={}&a={}".to_string(),
                _type: crate::rustlink::RustlinkType::LinkedIn,
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test%20multiple%20spaces%20test").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?q=multiple&a=spaces%20test"
        );
    }

    // TODO: additional URL encoding testss
}
