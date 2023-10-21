use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use etcd_rs::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::RustlinkAlias;
use crate::rustlink;

pub struct AppState {
    pub(crate) rustlinks: Arc<RwLock<HashMap<RustlinkAlias, rustlink::Rustlink>>>,
    pub(crate) revision: Arc<RwLock<i64>>,
    pub(crate) etcd_client: Arc<Client>,
    pub(crate) links_file: Arc<RwLock<Option<File>>>,
    pub(crate) primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerdeAppState {
    pub(crate) rustlinks: HashMap<RustlinkAlias, rustlink::Rustlink>,
    pub(crate) revision: i64,
}

impl AppState {
    pub async fn from(&self) -> SerdeAppState {
        let mut rustlinks: HashMap<RustlinkAlias, rustlink::Rustlink> = HashMap::new();

        let links = self.rustlinks.read().await;
        rustlinks.extend(links.clone());

        let revision = *self.revision.read().await;

        SerdeAppState {
            rustlinks,
            revision,
        }
    }
}
