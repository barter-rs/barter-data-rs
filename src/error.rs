use thiserror::Error;
use tokio_tungstenite::tungstenite;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to establish websocket connection due to failed websocket handshake")]
    WebSocketConnect(#[source] tungstenite::error::Error),

    #[error("Failed to write data via websocket connection")]
    WebSocketWrite(#[source] tungstenite::error::Error),

    #[error("Failed to read data via websocket connection")]
    WebSocketRead(#[source] tungstenite::error::Error),

    #[error("Failed to deserialize/serialize JSON due to: {0}")]
    JsonSerDeError(#[from] serde_json::Error),

    #[error("Failed to send message due to dropped receiver")]
    SendFailure,
}
