use std::str::FromStr;

use openidconnect::core::CoreClient;
use openidconnect::core::CoreProviderMetadata;
use openidconnect::reqwest::async_http_client;
use openidconnect::ClientId;
use openidconnect::ClientSecret;
use openidconnect::IssuerUrl;
use openidconnect::RedirectUrl;
use serde::{Deserialize, Serialize};

use crate::util::password_prompt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub client_id: String,
    pub client_secret: String,
    pub provider_url: String,
}

impl FromStr for OIDCProvider {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = value.split(",").map(|f| f.trim()).collect();

        if parts.len() < 2 || parts.len() > 3 {
            return Err("Invalid number of comma-separated values passed for OIDC provider");
        }

        let client_secret: String;
        let provider_url = String::from(parts[0]);
        let client_id = String::from(parts[1]);

        if parts.len() == 2 {
            let result =
                password_prompt(format!("OIDC client secret for {:?}:", provider_url).as_str());

            if result.is_ok() {
                client_secret = result.unwrap();
            } else {
                return Err("Failed to retrieve client_secret from OIDC input");
            }
        } else {
            client_secret = String::from(parts[2]);
        }

        Ok(OIDCProvider {
            client_id,
            provider_url,
            client_secret,
        })
    }
}

use crate::errors::RustlinksError;

pub async fn create_provider_client(
    provider: OIDCProvider,
    redirect_url: String,
) -> Result<CoreClient, RustlinksError> {
    let issuer_url = IssuerUrl::new(provider.provider_url)?;
    let client_id = ClientId::new(provider.client_id);
    let client_secret = ClientSecret::new(provider.client_secret);
    let redirect_url = RedirectUrl::new(redirect_url)?;

    match CoreProviderMetadata::discover_async(issuer_url, async_http_client).await {
        Ok(provider_metadata) => {
            let client = CoreClient::from_provider_metadata(
                provider_metadata,
                client_id,
                Some(client_secret),
            )
            .set_redirect_uri(redirect_url);

            Ok(client)
        }
        Err(e) => Err(RustlinksError::OIDCDiscoveryError(e.to_string())),
    }
}
