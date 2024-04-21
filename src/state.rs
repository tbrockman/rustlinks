use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use etcd_rs::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::RustlinkAlias;
#[cfg(feature = "oauth")]
use crate::oidc;
use crate::{rustlink, storage::RustlinkStore};

#[cfg(feature = "oauth")]
pub struct OAuthState {
    pub(crate) oauth_redirect_endpoint: String,
    pub(crate) oidc_providers: Arc<RwLock<Vec<oidc::provider::OIDCProvider>>>,
    pub(crate) login_path: String,
}

pub struct AppState {
    pub(crate) etcd_client: Arc<Client>,
    pub(crate) read_only: bool,
    pub(crate) rustlink_store: Arc<dyn RustlinkStore + Send + Sync>,
    #[cfg(feature = "ui")]
    pub(crate) js_source: Arc<RwLock<Option<String>>>,
}
