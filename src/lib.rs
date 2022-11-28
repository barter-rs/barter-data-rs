#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]
#![allow(type_alias_bounds)]

use crate::exchange::ExchangeId;
use barter_integration::{
    ExchangeStream, Transformer,
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream}
};
use futures::SinkExt;
use tokio::sync::mpsc;
use tracing::error;


///! # Barter-Data

// /// Core data structures to support consuming [`MarketStream`]s.
// ///
// /// eg/ `MarketEvent`, `PublicTrade`, etc.
// pub mod model;
// /// Contains `Subscriber` & `ExchangeMapper` implementations for specific exchanges.
// pub mod exchange;
// /// Initialises [`MarketStream`]s for an arbitrary number of exchanges using generic Barter
// /// [`Subscription`]s.
// pub mod builder;


/// Todo:
pub mod subscriber;
pub mod exchange;
pub mod model;
pub mod util;


// Todo:
//  - Search for todos and fix.
//  - Add tests from historical code we have on github as i've deleted a bunch of de tests
//  - Add logging in key places! debug too
//  - Impl validate for Subscription<Exchange, Kind>

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Exchange: Transformer> = ExchangeStream<
    WebSocketParser, WsStream, Exchange, Exchange::Output
>;

pub trait Identifier<T> {
    fn id() -> T;
}

/// Consume [`WsMessage`]s transmitted from the [`ExchangeTransformer`] and send them on to the
/// exchange via the [`WsSink`].
///
/// If an [`ExchangeTransformer`] is required to send responses to the exchange (eg/ custom pongs),
/// it can so by transmitting the responses to the  `mpsc::UnboundedReceiver<WsMessage>` owned by
/// this asynchronous distribution task. These are then sent to the exchange via the [`WsSink`].
/// This is required because an [`ExchangeTransformer`] is operating in a synchronous trait context,
/// and therefore cannot flush the [`WsSink`] without the [`futures:task::context`].
async fn distribute_responses_to_the_exchange(
    exchange: ExchangeId,
    mut ws_sink: WsSink,
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if barter_integration::protocol::websocket::is_websocket_disconnected(&error) {
                break;
            }

            // Log error only if WsMessage failed to send over a connected WebSocket
            error!(
                %exchange,
                %error,
                "failed to send ExchangeTransformer output message to the exchange via WsSink"
            );
        }
    }
}

// Todo:
// /// Test utilities for conveniently generating public [`MarketEvent`] types.
// pub mod test_util {
//     use crate::{
//         model::{Candle, DataKind, MarketEvent, PublicTrade},
//     };
//     use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
//     use chrono::Utc;
//     use std::ops::{Add, Sub};
//     use crate::model::exchange::ExchangeId;
//
//     /// Build a [`MarketEvent`] of [`DataKind::Trade`] with the provided [`Side`].
//     pub fn market_trade(side: Side) -> MarketEvent {
//         MarketEvent {
//             exchange_time: Utc::now(),
//             received_time: Utc::now(),
//             exchange: Exchange::from(ExchangeId::Binance),
//             instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
//             kind: DataKind::Trade(PublicTrade {
//                 id: "trade_id".to_string(),
//                 price: 1000.0,
//                 quantity: 1.0,
//                 side,
//             }),
//         }
//     }
//
//     /// Build a [`MarketEvent`] of [`DataKind::Candle`] with the provided time interval.
//     pub fn market_candle(interval: chrono::Duration) -> MarketEvent {
//         let now = Utc::now();
//         MarketEvent {
//             exchange_time: now,
//             received_time: now.add(chrono::Duration::milliseconds(200)),
//             exchange: Exchange::from(ExchangeId::Binance),
//             instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
//             kind: DataKind::Candle(Candle {
//                 start_time: now.sub(interval),
//                 end_time: now,
//                 open: 960.0,
//                 high: 1100.0,
//                 low: 950.0,
//                 close: 1000.0,
//                 volume: 100000.0,
//                 trade_count: 1000,
//             }),
//         }
//     }
// }
