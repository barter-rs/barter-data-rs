use thiserror::Error;

/// All errors related to Coinbase REST responses and websocket messages.
#[derive(Debug, Error)]
pub enum CoinbaseMsgError {
    #[error("Order message is missing a sequence.")]
    NoSequence,

    #[error("Order message is missing a type.")]
    NoType,

    #[error("Order message is missing an order id.")]
    NoOrderID,

    #[error("Invalid open order message is missing a key field")]
    InvalidOpen,

    #[error("Invalid change order message is missing a key field")]
    InvalidUpdate,

    #[error("Unhandled order message type: {0}")]
    UnhandledType(String),
}

