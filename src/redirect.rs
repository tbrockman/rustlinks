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
/// Assumes that the params we receive are % decoded.
pub fn template_params(template: &str, remaining: Option<&str>) -> String {
    let mut hat_replacement_indices: Vec<usize> = Vec::new();
    let mut parentheses_indices: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut result: Vec<String> = Vec::new();

    let mut i = 0;

    for char in template.chars() {
        match char {
            '{' => {
                if stack.len() == 0 {
                    parentheses_indices.push(i)
                }
                stack.push(i)
            }
            '}' => {
                // If '}' closes all open parentheses
                if let Some(_) = stack.pop() && stack.len() == 0 {
                    parentheses_indices.push(i)
                }
            }
            '^' => {
                // If '^' contained within {}
                if stack.len() > 0 {
                    hat_replacement_indices.push(i)
                }
            }
            _ => {}
        };

        result.push(char.to_string());
        i = i + 1;
    }

    println!("replace hats: {:?}", hat_replacement_indices);
    println!("replace parens: {:?}", parentheses_indices);
    println!("string vec before replacement: {:?}", result);

    let mut split = remaining.unwrap_or("").split(" ");
    let mut iter = hat_replacement_indices.iter();

    while let Some(indice) = iter.next() {
        println!("replacing indice: {:?}", indice);

        match split.next() {
            // If we have a param to replace, replace
            Some(param) => {
                result[*indice] = encode(param).to_string();
            }
            // Otherwise, replace with empty string
            None => {
                result[*indice] = "".to_string();
            }
        };
    }

    if let Some(remainder) = split.remainder() {
        if remainder.len() > 0 {
            result.push(encode(format!(" {remainder}").as_str()).to_string());
        }
    }

    iter = parentheses_indices.iter();

    // process parentheses
    while let Some(indice) = iter.next() {
        // check whether there is a replacement for the corresponding '^'

        // if not, set everything inbetween parentheses to ""

        println!("removing parans at indice: {:?}", indice);
        result[*indice] = "".to_string();
    }

    // build string
    result.join("")
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
        let url = "https://google.com/search?q={^}";
        let params = Some("rust");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust");
    }

    #[test]
    fn it_templates_string_with_multiple_spaces_url_encoded() {
        let url = "https://google.com/search?q={^}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust%20is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements() {
        let url = "https://google.com/search?q={^}&b={^}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements_with_fewer_params() {
        let url = "https://google.com/search?q={^}&a={^}&b={^}&c={^}&d={^}";
        let params = Some("rust is cool");
        let templated = template_params(url, params);
        assert_eq!(
            templated,
            "https://google.com/search?q=rust&a=is&b=cool&c=&d="
        );
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available() {
        let url = "https://google.com/search?q={shouldbehere%20^}&a={shouldnotbehere%20^}";
        let params = Some("rust");
        let templated = template_params(url, params);
        assert_eq!(
            templated,
            "https://google.com/search?q=shouldbehere%20rust&a="
        );
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available_test_encoding() {
        let url = "https://google.com{/search?q=^&b=test}";
        let params = Some("rust");
        let templated = template_params(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=test");
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
