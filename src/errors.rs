use std;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;

use actix_web;
use etcd_rs;

#[derive(Debug)]
pub enum StartError {
    EtcdError(etcd_rs::Error),
    ActixError(actix_web::Error),
    IoError(std::io::Error),
}

impl Error for StartError {}

impl Display for StartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::EtcdError(e) => write!(f, "etcd error: {:?}", e),
            StartError::ActixError(e) => write!(f, "actix error: {:?}", e),
            StartError::IoError(e) => write!(f, "io error: {:?}", e),
        }
    }
}

impl From<etcd_rs::Error> for StartError {
    fn from(e: etcd_rs::Error) -> Self {
        StartError::EtcdError(e)
    }
}

impl From<actix_web::Error> for StartError {
    fn from(e: actix_web::Error) -> Self {
        StartError::ActixError(e)
    }
}

impl From<std::io::Error> for StartError {
    fn from(e: std::io::Error) -> Self {
        StartError::IoError(e)
    }
}
