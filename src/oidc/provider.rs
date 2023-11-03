use std::{collections::HashMap, str::FromStr};

use lazy_static::lazy_static;
use openidconnect::core::CoreProviderMetadata;
use openidconnect::reqwest::async_http_client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub client_id: String,
    pub provider_url: String,
    pub provider_name: ProviderName,
    pub provider_metadata: Option<openidconnect::core::CoreProviderMetadata>,
}

impl FromStr for OIDCProvider {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = value.split(",").map(|f| f.trim()).collect();

        if parts.len() != 2 {
            return Err(
                "Invalid number of comma-separated values passed for OIDC provider (expected: 2)",
            );
        }
        let provider_url = String::from(parts[0]);
        let client_id = String::from(parts[1]);
        let provider_name = ProviderName::from_str(&provider_url);

        Ok(OIDCProvider {
            client_id,
            provider_url,
            provider_name,
            provider_metadata: None,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProviderName {
    Apple,
    Facebook,
    Google,
    Microsoft,
    Okta,
    Slack,
    Unknown,
}

lazy_static! {
    static ref URL_TO_PROVIDER: HashMap<String, ProviderName> = HashMap::from([
        (
            "https://www.facebook.com".to_string(),
            ProviderName::Facebook,
        ),
        ("https://facebook.com".to_string(), ProviderName::Facebook,),
        (
            "https://accounts.google.com".to_string(),
            ProviderName::Google,
        ),
        (
            "https://login.microsoftonline.com/common/v2.0".to_string(),
            ProviderName::Microsoft,
        ),
        ("https://login.okta.com".to_string(), ProviderName::Okta,),
        ("https://slack.com".to_string(), ProviderName::Slack,),
        ("https://appleid.apple.com".to_string(), ProviderName::Apple,),
    ]);
}

impl ProviderName {
    fn from_str(input: &str) -> Self {
        let trailing_removed = input.trim_end_matches('/').to_string();

        if let Some(provider) = URL_TO_PROVIDER.get(&trailing_removed) {
            return provider.clone();
        } else {
            if let Ok(url) = url::Url::parse(&trailing_removed) && url.host_str().unwrap_or_default().ends_with("okta.com") {
                return ProviderName::Okta;
            } else {
                return ProviderName::Unknown;
            }
        }
    }
}

pub async fn populate_provider_metadata(providers: Vec<OIDCProvider>) -> Vec<OIDCProvider> {
    futures::future::join_all(providers.into_iter().map(async move |mut p| {
        p.provider_url = p.provider_url.trim_end_matches('/').to_string();
        let issuer_url = openidconnect::IssuerUrl::new(p.provider_url.clone()).unwrap();
        let metadata_result =
            CoreProviderMetadata::discover_async(issuer_url, async_http_client).await;
        if let Err(e) = &metadata_result {
            println!("Error discovering provider metadata: {}", e);
        }
        p.provider_metadata = metadata_result.ok();
        p
    }))
    .await
}

#[cfg(test)]
mod unit_tests {
    #[test]
    fn test_provider_name_from_string() {
        use super::*;

        let test_case_result_vec = vec![
            ("https://www.facebook.com/", ProviderName::Facebook),
            ("https://facebook.com/", ProviderName::Facebook),
            (
                "https://login.microsoftonline.com/common/v2.0/",
                ProviderName::Microsoft,
            ),
            (
                "https://login.microsoftonline.com/common/v2.0",
                ProviderName::Microsoft,
            ),
            ("https://login.okta.com/", ProviderName::Okta),
            ("https://customdomain.okta.com", ProviderName::Okta),
            ("https://slack.com/", ProviderName::Slack),
            ("https://appleid.apple.com/", ProviderName::Apple),
            ("https://notonthelist.com/", ProviderName::Unknown),
            ("notevenavalidurl", ProviderName::Unknown),
        ];

        test_case_result_vec
            .into_iter()
            .for_each(|(input, expected)| {
                let actual = ProviderName::from_str(input);
                assert_eq!(actual, expected);
            });
    }
}
