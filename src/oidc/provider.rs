use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub client_id: String,
    pub provider_url: String,
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

        Ok(OIDCProvider {
            client_id,
            provider_url,
        })
    }
}
