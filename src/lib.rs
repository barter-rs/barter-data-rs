pub mod client;
pub mod connection;
pub mod error;
pub mod model;

use crate::error::ClientError;
use crate::model::{Candle, Trade};
use async_trait::async_trait;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::debug;

// Todo: general:
//  - Increase test coverage significantly now you know the PoC design works
//  - Unsure all .unwrap()s have been exchanged for more robust handling
//  - Ensure proper error handling & swapping unwraps() for more robust handling
//     '-> ensure all methods are returning an appropriate Result which is handled by caller

// Todo: connection.rs:
//  - Improve method of confirming subscription request so test_subscribe unit test passed
//     '-> subscription succeeded even if it didn't, need to confirm first message arrives?
//     '-> ensure logging is aligned once this has been done
//  - manage() add in connection fixing, reconnections

/// Useful type alias for a [WebSocketStream] connection.
pub type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Client trait defining the behaviour of all implementing ExchangeClients. All methods return
/// a stream of normalised data.
#[async_trait]
pub trait ExchangeClient {
    async fn consume_trades(
        &mut self,
        symbol: String,
    ) -> Result<UnboundedReceiverStream<Trade>, ClientError>;
    async fn consume_candles(
        &mut self,
        symbol: String,
        interval: &str,
    ) -> Result<UnboundedReceiverStream<Candle>, ClientError>;
}

/// Utilised to subscribe to an exchange's [WebSocketStream] via a ConnectionHandler (eg/ Trade stream).
pub trait Subscription {
    /// Constructs a new [Subscription] implementation.
    fn new(stream_name: String, ticker_pair: String) -> Self;
    /// Serializes the [Subscription] in a String data format.
    fn as_text(&self) -> Result<String, ClientError>
    where
        Self: Serialize,
    {
        Ok(serde_json::to_string(self)?)
    }
}

/// Returns a stream identifier that can be used to route messages from a [Subscription].
pub trait StreamIdentifier {
    fn get_stream_id(&self) -> Identifier;
}

/// Enum returned from [StreamIdentifier] representing if a struct has an identifiable stream Id.
pub enum Identifier {
    Yes(String),
    No,
}

/// Connect asynchronously to an exchange's server, returning a [WebSocketStream].
async fn connect(base_uri: &String) -> Result<WSStream, ClientError> {
    debug!("Establishing WebSocket connection to: {:?}", base_uri);
    connect_async(base_uri)
        .await
        .and_then(|(ws_stream, _)| Ok(ws_stream))
        .map_err(|err| ClientError::WebSocketConnect(err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        struct TestCase {
            input_base_uri: String,
            expected_can_connect: bool,
        }

        let test_cases = vec![
            TestCase {
                // Test case 0: Not a valid WS base URI
                input_base_uri: "not a valid base uri".to_string(),
                expected_can_connect: false,
            },
            TestCase {
                // Test case 1: Valid Binance WS base URI
                input_base_uri: "wss://stream.binance.com:9443/ws".to_string(),
                expected_can_connect: true,
            },
            TestCase {
                // Test case 2: Valid Bitstamp WS base URI
                input_base_uri: "wss://ws.bitstamp.net/".to_string(),
                expected_can_connect: true,
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            let actual_result = connect(&test.input_base_uri).await;
            assert_eq!(
                test.expected_can_connect,
                actual_result.is_ok(),
                "Test case: {:?}",
                index
            );
        }
    }
}
