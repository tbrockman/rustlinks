use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
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

#[derive(Args, Debug)]
pub struct GlobalOpts {
    /// Optional path to a .toml config file (precendence = args > env vars >
    /// config file)
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,

    /// Turn debug logging on
    #[arg(short, long)]
    pub(crate) debug: bool,

    /// Hostname(s) or IP address(es) of the etcd server(s)
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
    #[arg(long, default_value_t = true)]
    pub(crate) primary: bool,

    /// Certificate file to be used by the server for TLS
    #[arg(long)]
    pub(crate) cert_file: Option<PathBuf>,

    /// Key file to be used by the server for TLS
    #[arg(long)]
    pub(crate) key_file: Option<PathBuf>,

    /// OpenTelemetry collector endpoint
    #[arg(long, default_value = "http://127.0.0.1:4317")]
    pub(crate) otel_collector_endpoint: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the server
    Start {
        /// Hostname or IP address to bind to
        #[arg(long, default_value = "127.0.0.1")]
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
    /// the application to run successfully
    Configure {},
}
