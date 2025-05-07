#![feature(str_split_remainder)]
#![feature(let_chains)]
#![feature(async_closure)]
#![feature(const_trait_impl)]
#![feature(stmt_expr_attributes)]

pub mod api;
pub mod cli;
pub mod errors;
pub mod oidc;
pub mod redirect;
pub mod rustlink;
pub mod state;
pub mod storage;
pub mod tls;
pub mod ui;
pub mod util;
pub mod worker;

mod commands;

#[tokio::main]
async fn main() -> Result<(), errors::RustlinksError> {
    let cli = cli::RustlinksOpts::parse();
    cli.run().await
}
