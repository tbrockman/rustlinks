use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

/// A simple application for managing short links
/// For debug logs, set RUST_LOG=debug
#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct RustlinksOpts {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Commands,
}

impl RustlinksOpts {
    pub fn parse() -> Self {
        RustlinksOpts::parse_from(std::env::args())
    }
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct GlobalOpts {
    // /// Optional path to a .toml config file (precendence = args > env vars >
    // /// config file)
    // #[arg(long)]
    // pub(crate) config: Option<PathBuf>,
    /// Hostname(s) or IP address(es) of the etcd server(s), comma-separated if
    /// using multiple
    #[arg(
        long,
        use_value_delimiter = true,
        default_value = "http://127.0.0.1:2379"
    )]
    pub(crate) etcd_endpoints: Option<String>,

    /// Path to CA certificate to be used for communication with etcd (if
    /// passed, TLS will be used)
    #[arg(long)]
    pub(crate) etcd_ca_cert: Option<PathBuf>,

    /// Username to use for etcd authentication
    #[arg(long, default_value = "rustlinks")]
    pub(crate) etcd_username: Option<String>,

    /// Password to use for etcd authentication
    #[arg(long, default_value = "admin")]
    pub(crate) etcd_password: Option<String>,

    /// Flag to indicate whether server should run as primary (clients are
    /// secondary)
    /// Primary server will be responsible for writing to etcd
    /// Secondary servers are read-only, and will forward unfulfillable requests
    /// to the primary as a fallback.
    #[arg(long, default_value_t = false)]
    pub(crate) primary: bool,

    /// Certificate file to be used by the server for TLS
    #[arg(long)]
    pub(crate) cert_file: Option<PathBuf>,

    /// Key file to be used by the server for TLS
    #[arg(long)]
    pub(crate) key_file: Option<PathBuf>,

    /// OpenTelemetry collector endpoint
    #[arg(long, default_value = "http://127.0.0.1:4317")]
    pub(crate) otel_collector_endpoint: Option<String>,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Commands {
    /// Start the server
    Start {
        /// Hostname or IP address to bind to
        #[arg(long, default_value = "0.0.0.0")]
        hostname: String,

        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Path to a directory to persist Rustlink data
        #[arg(long, default_value = ".rustlinks/")]
        data_dir: PathBuf,
    },
    /// Configure the application, automatically performs certificate
    /// generation, role provisioning, and other setup required for
    /// the application to run
    Configure {
        /// etcd read-only user to create (if it doesn't already exist)
        #[arg(long, default_value = "rustlinks_ro")]
        etcd_readonly_user: String,

        /// etcd read-only user password
        #[arg(long, default_value = "default")]
        etcd_readonly_password: String,

        /// Use `mkcert` to create a local CA to generate certificates
        /// to provide TLS during navigation
        #[arg(long, default_value_t = true)]
        use_mkcert: bool,
    },
}

#[cfg(test)]
mod unit_tests {
    #[test]
    fn test_serialization() {
        use super::*;
        let opts = RustlinksOpts {
            global: GlobalOpts {
                etcd_endpoints: Some("http://".to_string()),
                etcd_ca_cert: None,
                etcd_username: None,
                etcd_password: None,
                primary: true,
                cert_file: None,
                key_file: None,
                otel_collector_endpoint: Some("http://".to_string()),
            },
            command: Commands::Start {
                hostname: "".to_string(),
                port: 0,
                data_dir: PathBuf::from(""),
            },
        };
        let serialized = serde_json::to_string(&opts).unwrap();
        println!("{}", serialized);
    }
}
