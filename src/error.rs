use crate::SubscriptionId;
use barter_integration::socket::error::SocketError;
use thiserror::Error;

/// All data related errors generated in `barter-data`.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("socket error: {0}")]
    Socket(#[from] SocketError),

    #[error("error subscribing to resources over the socket: {0}")]
    Subscribe(String),

    #[error("{entity} does not support: {item}")]
    Unsupported { entity: &'static str, item: String },

    #[error("consumed unidentifiable message: {0}")]
    Unidentifiable(SubscriptionId),
}