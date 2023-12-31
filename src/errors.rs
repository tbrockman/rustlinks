use std;

use etcd_rs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RustlinksError {
    #[error("etcd error: {0}")]
    EtcdError(#[from] etcd_rs::Error),
    #[error("actix error: {0}")]
    ActixError(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("trace error: {0}")]
    TraceError(#[from] opentelemetry::trace::TraceError),
    #[error("metrics error: {0}")]
    MetricsError(#[from] opentelemetry::metrics::MetricsError),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("oidc discovery error: {0}")]
    OIDCDiscoveryError(String),
    #[error("oauth endpoint parse error: {0}")]
    OAuthEndpointParseError(#[from] url::ParseError),
    #[error("unknown oidc provider: {0}")]
    UnknownOIDCProvider(String),
}
