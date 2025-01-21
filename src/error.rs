use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    SendError(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    Error(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
