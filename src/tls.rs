use std::path::PathBuf;
use std::{fs::File, io::BufReader};

use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

use crate::errors::RustlinksError;

// TODO: allow client auth
// TODO: allow SNI
pub fn load_rustls_config(
    cert_file_path: PathBuf,
    key_file_path: PathBuf,
) -> Result<rustls::ServerConfig, RustlinksError> {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = File::open(cert_file_path)?;
    let key_file = File::open(key_file_path.clone())?;
    let cert_buf = &mut BufReader::new(cert_file);
    let key_buf = &mut BufReader::new(key_file);

    // convert files to key/cert objects
    let cert_chain = certs(cert_buf)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_buf)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
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
