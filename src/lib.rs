#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data

use crate::model::{MarketEvent, StreamId, Subscription};
use barter_integration::socket::{
    {ExchangeSocket, Transformer},
    error::SocketError,
    protocol::websocket::{connect, ExchangeWebSocket, WebSocketParser, WsMessage},
};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use futures::{SinkExt, Stream};
use async_trait::async_trait;

pub mod builder;
pub mod model;
pub mod binance;


/// Todo:
pub trait StreamIdentifier {
    fn to_stream_id(&self) -> StreamId;
}

/// Todo:
#[async_trait]
pub trait MarketStream: Stream<Item = Result<MarketEvent, SocketError>> + Sized + Unpin {
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError>;
}

/// Todo:
pub trait ExchangeTransformer: Sized
where
    Self: Transformer<MarketEvent>,
{
    const EXCHANGE: ExchangeId;
    const BASE_URL: &'static str;
    fn new() -> Self;
    fn generate_subscriptions(&mut self, subscriptions: &[Subscription]) -> Vec<serde_json::Value>;
}

#[async_trait]
impl<ExchangeT> MarketStream for ExchangeWebSocket<ExchangeT, ExchangeT::Input, MarketEvent>
where
    Self: Stream<Item = Result<MarketEvent, SocketError>> + Sized + Unpin,
    ExchangeT: ExchangeTransformer + Send,
{
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError> {
        // Construct Exchange Transformer to translate between Barter & exchange data structures
        let mut exchange = ExchangeT::new();

        // Connect to exchange WebSocket server
        let mut websocket = connect(ExchangeT::BASE_URL).await?;

        // Action Subscriptions over the socket
        for sub_payload in exchange.generate_subscriptions(subscriptions) {
            websocket
                .send(WsMessage::Text(sub_payload.to_string()))
                .await?;
        }

        Ok(ExchangeSocket::new(websocket, WebSocketParser, exchange))
    }
}

// Todo: Rust docs, add basic impls, change name to ExchangeId ? Produce &'static str
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum ExchangeId {
    BinanceFutures,
    Binance,
    Ftx,
}

impl ExchangeId {
    /// Todo:
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::Binance => "binance",
            ExchangeId::BinanceFutures => "binance_futures",
            ExchangeId::Ftx => "ftx",
        }
    }

    /// Todo:
    pub fn supports_spot(&self) -> bool {
        match self {
            ExchangeId::BinanceFutures => false,
            _ => true,
        }
    }

    /// Todo:
    pub fn supports_futures(&self) -> bool {
        match self {
            ExchangeId::BinanceFutures => true,
            _ => false,
        }
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ExchangeId::BinanceFutures => "binance_futures",
            ExchangeId::Binance => "binance",
            ExchangeId::Ftx => "ftx",
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use super::*;
    use crate::builder::Streams;
    use crate::model::StreamKind;
    use barter_integration::InstrumentKind;

    // Todo: Add subscription validation - it currently fails silently
    // Todo: Maybe OutputIter will become an Option<OutputIter>?
    // Todo: Do I want to keep the name trait Exchange? Do I like the generic ExTransformer, etc.

    #[tokio::test]
    async fn stream_builder_works() -> Result<(), Box<dyn std::error::Error>> {

        let streams = Streams::builder()
            .subscribe(ExchangeId::BinanceFutures, [
                ("btc", "usdt", InstrumentKind::FuturePerpetual, StreamKind::Trades),
                ("eth", "usdt", InstrumentKind::FuturePerpetual, StreamKind::Trades),
            ])
            // .subscribe(ExchangeId::Binance, [
            //     ("btc", "usdt", InstrumentKind::Spot, StreamKind::Trades),
            //     ("eth", "usdt", InstrumentKind::Spot, StreamKind::Trades),
            // ])
            // .subscribe(ExchangeId::Ftx, [
            //     ("btc", "usdt", InstrumentKind::Spot, StreamKind::Trades),
            //     ("eth", "usdt", InstrumentKind::Spot, StreamKind::Trades),
            // ])
            .init()
            .await?;

        // Select individual exchange streams
        // let mut futures_stream = streams
        //     .select(ExchangeId::BinanceFutures)
        //     .ok_or(SocketError::Unidentifiable("".to_owned()))?; // Todo


        // let mut ftx_stream = streams
        //     .select(ExchangeId::Ftx)
        //     .ok_or(SocketError::Unidentifiable("".to_owned()))?; // Todo:

        // Join the remaining exchange streams into one
        let mut joined_stream = streams.join().await;

        while let Some((exchange, event)) = joined_stream.next().await {
            println!("{:?}", event);
        }


        Ok(())
    }
}



















// pub mod test_util {
//     use crate::model::Candle;
//     use chrono::Utc;
//
//     pub fn candle() -> Candle {
//         Candle {
//             start_timestamp: Utc::now(),
//             end_timestamp: Utc::now(),
//             open: 1000.0,
//             high: 1100.0,
//             low: 900.0,
//             close: 1050.0,
//             volume: 1000000000.0,
//             trade_count: 100,
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[tokio::test]
//     async fn test_connect() {
//         struct TestCase {
//             input_base_uri: String,
//             expected_can_connect: bool,
//         }
//
//         let test_cases = vec![
//             TestCase {
//                 // Test case 0: Not a valid WS base URI
//                 input_base_uri: "not a valid base uri".to_string(),
//                 expected_can_connect: false,
//             },
//             TestCase {
//                 // Test case 1: Valid Binance WS base URI
//                 input_base_uri: "wss://stream.binance.com:9443/ws".to_string(),
//                 expected_can_connect: true,
//             },
//             TestCase {
//                 // Test case 2: Valid Bitstamp WS base URI
//                 input_base_uri: "wss://ws.bitstamp.net/".to_string(),
//                 expected_can_connect: true,
//             },
//         ];
//
//         for (index, test) in test_cases.into_iter().enumerate() {
//             let actual_result = connect(&test.input_base_uri).await;
//             assert_eq!(
//                 test.expected_can_connect,
//                 actual_result.is_ok(),
//                 "Test case: {:?}",
//                 index
//             );
//         }
//     }
// }