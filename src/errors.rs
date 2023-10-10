use std;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;

use actix_web;
use etcd_rs;

#[derive(Debug)]
pub enum RustlinksError {
    EtcdError(etcd_rs::Error),
    ActixError(actix_web::Error),
    IoError(std::io::Error),
    TraceError(opentelemetry::trace::TraceError),
    MetricsError(opentelemetry::metrics::MetricsError),
}

impl Error for RustlinksError {}

impl Display for RustlinksError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RustlinksError::EtcdError(e) => write!(f, "etcd error: {:?}", e),
            RustlinksError::ActixError(e) => write!(f, "actix error: {:?}", e),
            RustlinksError::IoError(e) => write!(f, "io error: {:?}", e),
            RustlinksError::TraceError(e) => write!(f, "trace error: {:?}", e),
            RustlinksError::MetricsError(e) => write!(f, "metrics error: {:?}", e),
        }
    }
}

impl From<etcd_rs::Error> for RustlinksError {
    fn from(e: etcd_rs::Error) -> Self {
        RustlinksError::EtcdError(e)
    }
}

impl From<actix_web::Error> for RustlinksError {
    fn from(e: actix_web::Error) -> Self {
        RustlinksError::ActixError(e)
    }
}

impl From<std::io::Error> for RustlinksError {
    fn from(e: std::io::Error) -> Self {
        RustlinksError::IoError(e)
    }
}

impl From<opentelemetry::trace::TraceError> for RustlinksError {
    fn from(e: opentelemetry::trace::TraceError) -> Self {
        RustlinksError::TraceError(e)
    }
}

impl From<opentelemetry::metrics::MetricsError> for RustlinksError {
    fn from(e: opentelemetry::metrics::MetricsError) -> Self {
        RustlinksError::MetricsError(e)
    }
}
