use std::path::PathBuf;
use std::{fs::File, io::BufReader};

use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};

use crate::errors::RustlinksError;

// TODO: support client auth
// TODO: support SNI
pub fn load_rustls_config(
    cert_file_path: PathBuf,
    key_file_path: PathBuf,
) -> Result<rustls::ServerConfig, RustlinksError> {
    // init server config builder with safe defaults
    let config = ServerConfig::builder().with_no_client_auth();

    // load TLS key/cert files
    let cert_file = File::open(cert_file_path)?;
    let key_file = File::open(key_file_path.clone())?;
    let cert_buf = &mut BufReader::new(cert_file);
    let key_buf = &mut BufReader::new(key_file);

    // convert files to key/cert objects
    let cert_chain = certs(cert_buf).into_iter().filter_map(|c| c.ok()).collect();
    let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(key_buf)
        .into_iter()
        .filter_map(|k| {
            if let Ok(key) = k {
                Some(PrivateKeyDer::from(key))
            } else {
                None
            }
        })
        .collect();

    match keys.is_empty() {
        true => Err(RustlinksError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No private keys found in file: {:?}", key_file_path),
        ))),
        false => match config.with_single_cert(cert_chain, keys.remove(0)) {
            Ok(config) => Ok(config),
            Err(e) => Err(RustlinksError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error loading TLS key/cert files: {:?}", e),
            ))),
        },
    }
}
