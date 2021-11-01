pub mod connection;
pub mod error;
pub mod client;

use crate::error::ClientError;
use log::info;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::net::TcpStream;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

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
    async fn consume_trades(&mut self, symbol: String) -> Result<UnboundedReceiverStream<Trade>, ClientError>;
    async fn consume_candles(&mut self, symbol: String, interval: &str) -> Result<UnboundedReceiverStream<Candle>, ClientError>;
}

/// Utilised to subscribe to an exchange's [WebSocketStream] via a ConnectionHandler (eg/ Trade stream).
pub trait Subscription {
    /// Constructs a new [Subscription] implementation.
    fn new(stream_name: String, ticker_pair: String) -> Self;
    /// Serializes the [Subscription] in a String data format.
    fn as_text(&self) -> Result<String, ClientError> where Self: Serialize {
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
    info!("Establishing WebSocket connection to: {:?}", base_uri);
    connect_async(base_uri)
        .await
        .and_then(|(ws_stream, _)| Ok(ws_stream))
        .map_err(|err| ClientError::WebSocketConnect(err))
}

/// Normalised Trade model to be returned from an [ExchangeClient].
#[derive(Debug, Deserialize, Serialize)]
pub struct Trade {
    trade_id: String,
    timestamp: String,
    ticker: String,
    price: f64,
    quantity: f64,
    buyer: BuyerType,
}

/// Defines if the buyer in a [Trade] is a market maker.
#[derive(Debug, Deserialize, Serialize)]
pub enum BuyerType {
    MarketMaker,
    Taker,
}

/// Defines the possible intervals that a [Candle] represents.
#[derive(Debug, Deserialize, Serialize)]
pub enum Interval {
    Minute1, Minute3, Minute5, Minute15, Minute30,
    Hour1, Hour2, Hour4, Hour6, Hour8, Hour12,
    Day1, Day3,
    Week1,
    Month1,
}

/// Normalised OHLCV data from an [Interval] with the associated [DateTime] UTC timestamp;
#[derive(Debug, Deserialize, Serialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
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
            TestCase { // Test case 0: Not a valid WS base URI
                input_base_uri: "not a valid base uri".to_string(),
                expected_can_connect: false,
            },
            TestCase { // Test case 1: Valid Binance WS base URI
                input_base_uri: "wss://stream.binance.com:9443/ws".to_string(),
                expected_can_connect: true,
            },
            TestCase { // Test case 2: Valid Bitstamp WS base URI
                input_base_uri: "wss://ws.bitstamp.net/".to_string(),
                expected_can_connect: true
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            let actual_result = connect(&test.input_base_uri).await;
            assert_eq!(test.expected_can_connect, actual_result.is_ok(), "Test case: {:?}", index);
        };
    }
}
