use crate::{cli::RustlinksOpts, errors::RustlinksError};

pub async fn install(cli: RustlinksOpts) -> Result<(), RustlinksError> {
    // 1. install mkcert if necessary
    // TODO: replace with rust-specific library (or create one which just invokes necessary platform steps)
    // 2. modify /etc/hosts programmatically
    Ok(())
}
