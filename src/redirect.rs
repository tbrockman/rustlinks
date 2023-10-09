use actix_web::{get, web, Either, HttpResponse};
use dyn_fmt::AsStrFormatExt;
use urlencoding::encode;

use crate::state;

#[get("/{alias:.*}")]
pub async fn redirect(
    data: web::Data<state::AppState>,
    path: web::Path<String>,
) -> Either<web::Redirect, HttpResponse> {
    let full = path.into_inner();
    let mut split = full.split(" ");
    let alias = split.next().unwrap();
    let params = split.remainder();

    println!("full: {}", full);
    println!("alias: {}", alias);
    println!("params: {:?}", params);
    println!("have links: {:?}", data.rustlinks.read().await);
    let guard = data.rustlinks.read().await;
    match guard.get(alias) {
        Some(rustlink) => {
            let url = template_params(&rustlink.url, params);
            Either::Left(web::Redirect::to(url).permanent())
        }
        None => Either::Right(HttpResponse::NotFound().finish()),
    }
}

/// Take any params we received, and template them into the URL.
/// Assumes that the params we receive are already % decoded.
pub fn template_params(template: &str, remaining: Option<&str>) -> String {
    let mut params: Vec<String> = Vec::new();
    let matches = template.matches("{}").count();
    let mut split = remaining.unwrap_or("").split(" ");

    while matches > 0 && params.len() < matches - 1 {
        let param = split.next();
        params.push(encode(param.unwrap_or("")).to_string());
    }

    if let Some(remainder) = split.remainder() {
        params.push(encode(remainder).to_string());
    }
    template.format(&params)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn it_templates_nothing() {
        let url = "https://google.com";
        let params = None;
        let templated = template_params(url, params);
        assert_eq!(templated, url);
    }

    #[test]
    fn it_templates_one_item() {
        let url = "https://google.com/search?q={}";
        let params = Some("rust");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust");
    }

    #[test]
    fn it_templates_string_with_multiple_spaces_url_encoded() {
        let url = "https://google.com/search?q={}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust%20is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements() {
        let url = "https://google.com/search?q={}&b={}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements_with_fewer_params() {
        let url = "https://google.com/search?q={}&a={}&b={}&c={}&d={}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(
            templated,
            "https://google.com/search?q=rust&a=is&b=cool&c=&d="
        );
    }
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
            },
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    rustlinks: Arc::new(RwLock::new(rustlinks)),
                    etcd_client: Arc::new(client),
                    links_file: Arc::new(RwLock::new(None)),
                    revision: Arc::new(RwLock::new(0)),
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
