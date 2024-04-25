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
    pub(crate) etcd_endpoints: String,

    /// Path to CA certificate to be used for communication with etcd (if
    /// passed, TLS will be used)
    #[arg(long)]
    pub(crate) etcd_ca_cert: Option<PathBuf>,

    /// Username to use for etcd read-write account
    #[arg(long, default_value = "rustlinks_rw")]
    pub(crate) etcd_username: Option<String>,

    /// Password to use for etcd read-write account
    #[arg(long, default_value = "default")]
    pub(crate) etcd_password: Option<String>,

    /// Flag to indicate whether server should run as read_only (alternative is
    /// read_write). Read-only servers can't write anything to `etcd` and may
    /// retain a smaller set of links in-memory. Read-write servers can write
    /// to `etcd`, and will retain the full set of links in-memory
    #[arg(long, default_value_t = false)]
    pub(crate) read_only: bool,

    /// OpenTelemetry collector endpoint
    #[arg(long, default_value = "http://127.0.0.1:4317")]
    pub(crate) otel_collector_endpoint: Option<String>,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Commands {
    /// Start the server
    Start {
        /// Hostname or IP address to bind to
        #[arg(long, default_value = "127.13.37.1")]
        hostname: String,

        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Path to Rustlink database on disk
        #[arg(long, default_value = ".rustlinks/data.db")]
        db_path: PathBuf,

        /// Max size of LMDB memory map in bytes
        #[arg(long, default_value = "104857600")]
        db_map_size: usize,

        /// Certificate .PEM to be used by the server for TLS
        /// Specify both '--cert-file' and '--key-file' to enable TLS
        #[arg(long, requires("key_file"))]
        cert_file: Option<PathBuf>,

        /// Key .PEM to be used by the server for TLS
        /// Specify both '--cert-file' and '--key-file' to enable TLS
        #[arg(long, requires("cert_file"))]
        key_file: Option<PathBuf>,

        /// TODO:
        #[arg(
            long,
            default_value = "https://rustlinks.theo.lol/api/v1/oauth/callback"
        )]
        #[cfg(feature = "oauth")]
        oauth_redirect_uri: String, // TODO: fix host vs path

        /// TODO:
        #[arg(long, default_value = "/login")]
        // TODO: same as above
        #[cfg(feature = "oauth")]
        login_path: String,

        /// Specify any OIDC providers the server will support using a
        /// comma-separated tuple of provider_url,client_id.
        ///
        /// Example: --oidc-providers "https://accounts.google.com,123456789"
        ///
        /// `/api/v1/rustlinks` endpoints will be guarded by OIDC authentication
        #[arg(long, num_args = 0..)]
        #[cfg(feature = "oauth")]
        oidc_providers: Vec<oidc::provider::OIDCProvider>,

        /// URL to redirect to if the server is unable to find a given Glean Rustlink
        #[arg(long)]
        #[cfg(feature = "glean")]
        glean_fallback_url: Option<String>,

        /// URL to redirect to if the server is unable to find a given LinkedIn Rustlink
        #[arg(long)]
        #[cfg(feature = "li")]
        li_fallback_url: Option<String>,

        /// URL to redirect to if the server is unable to find a given Tera Rustlink
        #[arg(long)]
        #[cfg(feature = "tera")]
        tera_fallback_url: Option<String>,
    },
    /// Setup the application, automatically performs certificate
    /// generation, etcd role+user provisioning, and other setup required for
    /// the application to run in a typical production setup.
    Install {
        /// IP address for the local server to bind to
        /// Should be unique localhost IP address to guarantee ports :443 and
        /// :80 are available.
        #[arg(long, default_value = "127.13.37.1")]
        ip_address: String,

        /// Hostname to be associated with the local IP address in /etc/hosts
        #[arg(long, default_value = "rs")]
        hostname: String,

        /// etcd admin account with ability to provision users and roles
        /// This will be used to create read-only and read-write users for the
        /// application
        #[arg(long, default_value = "root")]
        etcd_admin_username: String,

        /// etcd admin password
        #[arg(long, default_value = "password")]
        etcd_admin_password: String,

        /// etcd read-only user to create (if it doesn't already exist)
        /// This user will be used by secondary servers to read from etcd,
        #[arg(long, default_value = "rustlinks_ro")]
        etcd_read_only_username: String,

        /// etcd read-only user password
        #[arg(long, default_value = "default")]
        etcd_read_only_password: String,

        /// etcd read-write user to create (if it doesn't already exist)
        /// This user will be used by secondary servers to read from etcd,
        #[arg(long, default_value = "rustlinks_rw")]
        etcd_read_write_username: String,

        /// etcd read-write user password
        #[arg(long, default_value = "default")]
        etcd_read_write_password: String,
    },
}

#[cfg(test)]
mod unit_tests {
    #[test]
    fn test_serialization() -> anyhow::Result<()> {
        use super::*;
        let opts = RustlinksOpts {
            global: GlobalOpts {
                etcd_endpoints: "http://".to_string(),
                etcd_ca_cert: None,
                etcd_username: None,
                etcd_password: None,
                read_only: true,
                otel_collector_endpoint: Some("http://".to_string()),
            },
            command: Commands::Start {
                hostname: "".to_string(),
                port: 0,
                db_path: PathBuf::from(""),
                db_map_size: 0,
                cert_file: None,
                key_file: None,
                li_fallback_url: None,
                glean_fallback_url: None,
                tera_fallback_url: None,
            },
        };
        let serialized = serde_json::to_string(&opts)?;
        println!("{}", serialized);
        Ok(())
    }
}
