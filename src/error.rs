use thiserror::Error;
use barter_integration::error::SocketError;


#[derive(Debug, Error)]
pub enum DataError {
    #[error("SocketError: {0}")]
    Socket(#[from] SocketError)
}