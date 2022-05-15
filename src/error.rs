use barter_integration::socket::error::SocketError;
use thiserror::Error;

/// All data related errors generated in `barter-data`.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("socket error: {0}")]
    Socket(#[from] SocketError),
}