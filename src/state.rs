use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use etcd_rs::Client;

use super::RustlinkAlias;
use crate::rustlink;

pub struct AppState {
    pub(crate) rustlinks: Arc<RwLock<HashMap<RustlinkAlias, rustlink::Rustlink>>>,
    pub(crate) last_mod_revision: Arc<RwLock<i64>>,
    pub(crate) client: Arc<Client>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerdeAppState {
    pub(crate) rustlinks: HashMap<RustlinkAlias, rustlink::Rustlink>,
    pub(crate) last_mod_revision: i64,
}

impl From<AppState> for SerdeAppState {
    fn from(state: AppState) -> Self {
        let mut rustlinks: HashMap<RustlinkAlias, rustlink::Rustlink> = HashMap::new();

        if let Ok(links) = state.rustlinks.read() {
            rustlinks.extend(links.clone());
        }

        let mut last_mod_revision = 0;

        if let Ok(revision) = state.last_mod_revision.read() {
            last_mod_revision = *revision;
        }

        SerdeAppState {
            rustlinks,
            last_mod_revision,
        }
    }
}
