#![warn(
    // missing_debug_implementations,
    // missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]
#![allow(type_alias_bounds)]

use crate::{
    exchange::ExchangeId,
    model::{Market, MarketIter},
    subscriber::subscription::{SubKind, SubscriptionMap},
};
use barter_integration::model::SubscriptionId;
use barter_integration::{
    error::SocketError,
    model::Instrument,
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream},
    ExchangeStream, Transformer,
};
use exchange::ExchangeMeta;
use futures::SinkExt;
use serde::Deserialize;
use std::marker::PhantomData;
use tokio::sync::mpsc;
use tracing::error;

pub mod exchange;
pub mod model;
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

// Todo: Next
//  0. Implement Bitfinex trades using code from MR
//  1. Look into adding an OrderBook transformer that uses http

// Todo: Important & May Cause Large Changes
//  0. Add TradeId new type to barter-integration -> is there one in barter-execution to move?
//  1. SubscriptionId should probably contain a reference to a String... then use serde borrow
//   '-- the gats here will probably complicate generics so do it after simplifying...
//   '--> Identifier<&SubscriptionId> etc. or use Cow inside SubscriptionId like Exchange?

// Todo: Simplification of generics & design

// Todo: Nice To Have Once Complicated Is Over
//  0. Subscriber becomes generic rather than hard-coded WebSocket
//     '--> Same with SubscriptionMeta::WsMessage, etc
//  1. Impl validate for Subscription<Exchange, Kind> -> May require generic Exchange...
//  2. Is it possible to remove enum from KrakenMessage? Way to do this properly so I can remove Identifier<Option<SubscriptionId>>?

// Todo: Just before release
//  0. Logging in key place, including debug statements.
//  1. Search for todos & action.
//  2. Go through and add tests from develop branch for each part that I've left out (fine tooth comb)
//  3. Normalise code comments & check links on exchanges.
//  4. Access modifiers on everything
//  5. Add maximum derives on everything. #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
//  6. Uncomment clippy warnings at the top of this file & fix resulting warnings.
//  7. Test all exchanges again

// Todo: Front-End
//  0. Update builder, etc.

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Exchange: Transformer> =
    ExchangeStream<WebSocketParser, WsStream, Exchange, Exchange::Output>;

pub trait ExchangeIdentifier {
    fn exchange_id() -> ExchangeId;
}

pub trait Identifier<T> {
    fn id(&self) -> T;
}

pub struct ExchangeTransformer<Exchange, Kind, ExchangeEvent> {
    pub subscription_map: SubscriptionMap<Kind>,
    phantom: PhantomData<(Exchange, ExchangeEvent)>,
}

impl<Exchange, Kind, ExchangeEvent> Transformer
    for ExchangeTransformer<Exchange, Kind, ExchangeEvent>
where
    Exchange: ExchangeMeta<ExchangeEvent>,
    Kind: SubKind,
    ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, ExchangeEvent)>,
{
    type Input = ExchangeEvent;
    type Output = Market<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, event: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match event.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.subscription_map.find_instrument(&subscription_id) {
            Ok(instrument) => {
                MarketIter::<Kind::Event>::from((Exchange::exchange_id(), instrument, event)).0
            }
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            }
        }
    }
}

impl<Exchange, Kind, ExchangeEvent> ExchangeTransformer<Exchange, Kind, ExchangeEvent> {
    pub fn new(subscription_map: SubscriptionMap<Kind>) -> Self {
        Self {
            subscription_map,
            phantom: PhantomData::<(Exchange, ExchangeEvent)>::default(),
        }
    }
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
