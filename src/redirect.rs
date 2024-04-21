use actix_web::{
    get,
    web::{Data, Path, Redirect},
    Either, HttpResponse,
};
use anyhow::Result;
use heed::types::{SerdeBincode, Str};
use heed::Env;
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
};

use crate::{rustlink::Rustlink, state};

pub fn get_link_by_alias(db_env: &Env, alias: &str) -> Result<Option<Rustlink>> {
    let rtxn = db_env.read_txn()?;
    if let Some(db) = db_env.open_database::<Str, SerdeBincode<Rustlink>>(&rtxn, None)? {
        let value = db.get(&rtxn, alias)?;
        Ok(value)
    } else {
        Ok(None)
    }
}

#[get("/{alias:.*}")]
pub async fn redirect(
    state: Data<state::AppState>,
    path: Path<String>,
) -> Either<Redirect, HttpResponse> {
    let tracer = global::tracer("redirect");
    tracer.in_span("render-url-template-and-redirect", move |cx| {
        let full = path.into_inner();
        let mut split = full.split(" ");
        let alias = if let Some(a) = split.next() {
            a
        } else {
            return Either::Right(HttpResponse::BadRequest().finish());
        };

        match state.rustlink_store.get(alias) {
            Ok(Some(rustlink)) => {
                let params: Vec<&str> = split.collect();
                let url = if let Ok(u) = rustlink.render(params.clone()) {
                    u
                } else {
                    return Either::Right(HttpResponse::InternalServerError().finish());
                };

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
                let span = cx.span();
                span.set_attribute(opentelemetry::KeyValue::new(
                    "rustlinks.alias",
                    alias.to_string(),
                ));
                span.set_attribute(opentelemetry::KeyValue::new("rustlinks.url", url.clone()));
                span.set_attribute(opentelemetry::KeyValue::new(
                    "rustlinks.params",
                    params.join(" ").to_string(),
                ));

                Either::Left(Redirect::to(url).permanent())
            }
            Ok(None) => Either::Right(HttpResponse::NotFound().finish()),
            Err(e) => Either::Right(HttpResponse::InternalServerError().body(e.to_string())),
        }
    })
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use actix_web::{test, App};
    use etcd_rs::{Client, ClientConfig, Endpoint};
    use heed::EnvOpenOptions;

    use super::*;
    use crate::storage::{RustlinkStore, LMDB};
    use crate::{rustlink::Rustlink, state::AppState};

    #[actix_web::test]
    async fn it_templates_no_items_with_no_format_string() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q=abcdefg".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
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
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q={}".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
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
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q=abcdefg".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
                    read_only: true,
                }))
                .service(redirect),
        )
        .await;
        let req = test::TestRequest::with_uri("/test%20test=parameter").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_redirection());
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "https://google.com/search?test=parameter&q=abcdefg"
        );
    }

    #[actix_web::test]
    async fn it_templates_items_with_format_string_and_params() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q={^}".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
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
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q={^}".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
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
    async fn it_templates_multiple_input_parameters_and_replaces_carets() {
        let client = Client::connect(ClientConfig::new(vec![Endpoint::new(
            "http://localhost:2379",
        )]))
        .await
        .unwrap();
        let db_path = tempfile::tempdir().unwrap();
        let rustlink_store = Arc::new(LMDB::new(EnvOpenOptions::new().open(db_path).unwrap()));
        let rustlink = Rustlink {
            url: "https://google.com/search?q={^}&a={}".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };
        rustlink_store.as_ref().set("test", &rustlink).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(Data::new(AppState {
                    etcd_client: Arc::new(client),
                    rustlink_store,
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
            "https://google.com/search?q=multiple%20spaces%20test&a="
        );
    }

    // TODO: additional URL encoding testss
}
