use std::collections::HashMap;

use actix_web::{get, web, Either, HttpResponse};
use opentelemetry::{
    global,
    trace::{get_active_span, Tracer},
};
use urlencoding::encode;

use crate::state;

#[get("/{alias:.*}")]
pub async fn redirect(
    data: web::Data<state::AppState>,
    path: web::Path<String>,
) -> Either<web::Redirect, HttpResponse> {
    let tracer = global::tracer("redirect");
    tracer
        .in_span("render-url-template-and-redirect", async move |_| {
            let full = path.into_inner();
            let mut split = full.split(" ");
            let alias = split.next().unwrap();
            let params = split.remainder();
            let rustlinks = data.rustlinks.read().await;

            get_active_span(|span| match rustlinks.get(alias) {
                Some(rustlink) => {
                    let url = render_url_template(&rustlink.url, params.clone());
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
                    span.set_attribute(opentelemetry::KeyValue::new("rustlinks.url", url.clone()));
                    span.set_attribute(opentelemetry::KeyValue::new(
                        "rustlinks.params",
                        params.unwrap_or("").to_string(),
                    ));
                    Either::Left(web::Redirect::to(url).permanent())
                }
                None => Either::Right(HttpResponse::NotFound().finish()),
            })
        })
        .await
}

/// Take any params we received, and template them into the URL.
/// Assumes that the params we receive are % decoded.
pub fn render_url_template(template: &str, params: Option<&str>) -> String {
    // Indices of occurences of '^' in template, and closing parentheses position
    let mut hat_replacement_indices: Vec<(usize, usize)> = Vec::new();
    // HashMap of closing->opening parentheses positions
    let mut parentheses_indices: HashMap<usize, usize> = HashMap::new();
    // Stack containing tuple of either '{' or '^', and index of occurence in
    // template
    let mut stack: Vec<(char, usize)> = Vec::new();
    // A vec of strings, will contain the final string to be joined after all
    // replacements
    let mut result: Vec<String> = Vec::new();

    let mut i = 0;

    for char in template.chars() {
        match char {
            '{' => {
                stack.push((char, i));
            }
            '}' => {
                let mut temp: Vec<(usize, usize)> = Vec::new();

                // If '}' closes an open parentheses
                while let Some((stack_char, char_idx)) = stack.pop() {
                    match stack_char {
                        '^' => {
                            temp.push((char_idx, i));
                        }
                        '{' => {
                            parentheses_indices.insert(i, char_idx);
                            break;
                        }
                        _ => {}
                    };
                }

                hat_replacement_indices
                    .append(&mut temp.iter().rev().map(|(a, b)| (*a, *b)).collect())
            }
            '^' => {
                if stack.len() > 0 {
                    stack.push((char, i));
                }
            }
            _ => {}
        };

        result.push(char.to_string());
        i = i + 1;
    }

    // TODO: params should be None but instead is ""

    let mut split = params.unwrap_or("").split(" ");
    let mut iter = hat_replacement_indices.iter();

    while let Some((hat_idx, paren_idx)) = iter.next() {
        match split.next() {
            // If we have an empty string, or None, remove everything between the corresponding
            // parentheses
            Some("") | None => {
                if let Some(start_idx) = parentheses_indices.get(paren_idx) {
                    for idx in *start_idx..*paren_idx + 1 {
                        result[idx] = "".to_string()
                    }
                };
            }
            // If we have a param to replace, replace
            Some(param) => {
                result[*hat_idx] = encode(param).to_string();
            }
        };
    }

    if let Some(remainder) = split.remainder() {
        if remainder.len() > 0 {
            result.push(encode(format!(" {remainder}").as_str()).to_string());
        }
    }

    for (end_idx, start_idx) in parentheses_indices.iter() {
        result[*start_idx] = "".to_string();
        result[*end_idx] = "".to_string();
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
        let templated = render_url_template(url, params);
        assert_eq!(templated, url);
    }

    #[test]
    fn it_templates_one_item() {
        let url = "https://google.com/search?q={^}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust");
    }

    #[test]
    fn it_templates_string_with_multiple_spaces_url_encoded() {
        let url = "https://google.com/search?q={^}";
        let params = Some("rust is cool");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust%20is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements() {
        let url = "https://google.com/search?q={^}&b={^}";
        let params = Some("rust is cool");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=is%20cool");
    }

    #[test]
    fn it_templates_string_with_multiple_replacements_with_fewer_params() {
        let url = "https://google.com/search?q={^}&a={^}&b={^}&c={^}&d={^}";
        let params = Some("rust is cool");
        let templated = render_url_template(url, params);
        assert_eq!(
            templated,
            "https://google.com/search?q=rust&a=is&b=cool&c=&d="
        );
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available() {
        let url = "https://google.com/search?q={shouldbehere%20^}&a={shouldnotbehere%20^}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(
            templated,
            "https://google.com/search?q=shouldbehere%20rust&a="
        );
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available_test_encoding_available() {
        let url = "https://google.com{/search?q=^&b=test}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=test");
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available_test_encoding_not_available() {
        let url = "https://google.com{/search?q=^&b=test}";
        let params = None;
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com");
    }

    #[test]
    fn it_only_replaces_parentheses_if_param_is_available_mixed() {
        let url = "https://google.com{/search?q=^&b=test}{#^}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=test");
    }

    #[test]
    fn it_handles_nesting_for_whatever_reason() {
        let url = "https://google.com{/search?q=^{&b=^}}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com");
    }

    #[test]
    fn it_handles_nesting_in_stack_order_for_whatever_reason() {
        let url = "https://google.com{/search?q=^%20is{&b=^}}";
        let params = Some("cool rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust%20is&b=cool");
    }

    #[test]
    fn it_only_replaces_if_has_sufficient_params() {
        let url = "https://google.com{/search?q=^&b=^}";
        let params = Some("rust");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com");
    }

    #[test]
    fn it_replaces_with_sufficient_params() {
        let url = "https://google.com{/search?q=^&b=^}";
        let params = Some("rust is cool");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com/search?q=rust&b=is%20cool");
    }

    #[test]
    fn it_handles_one_whitespace_as_no_input() {
        let url = "https://google.com{/search?q=^&b=^}";
        let params = Some(" ");
        let templated = render_url_template(url, params);
        assert_eq!(templated, "https://google.com");
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
                    read_only: true,
                    oauth_redirect_endpoint: Arc::new("".to_string()),
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
