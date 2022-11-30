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
use barter_integration::{
    error::SocketError,
    ExchangeStream,
    model::Instrument,
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream}, Transformer,
};
use std::marker::PhantomData;
use serde::Deserialize;
use futures::SinkExt;
use tokio::sync::mpsc;
use tracing::error;
use barter_integration::model::SubscriptionId;
use exchange::ExchangeMeta;


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
//  - Kraken broken by heartbeat LOL - fix
//  - Gateio FuturePerpetual has two URLs for btc & usdt -> look into customisable URLs?
//   '--> Currently using different ExchangeIds... -> Ideally combine btc & usdt into `Future`
//   '--> channels are easily composable from channel + instrument type...
//   '--> Identifier<Channel> could become ChannelIdentifier and also be passed a sub eg/ fn (&self, sub: &Sub<Kind>)...
//  - Newtype for `PairSymbol(String)` with convienece methods for delimiters & casing :)
//  - Search for todos and fix.
//  - Uncommon clippy warnings at top of this file & fix lints
//  - Add tests from historical code we have on github as i've deleted a bunch of de tests
//  - Add logging in key places! debug too
//  - Impl validate for Subscription<Exchange, Kind>
//  - Subscriber becomes generic rather than hard-coded WebSocket
//   '--> Same with SubscriptionMeta::WsMessage, etc
//  - Try to remove WebSocketSubscriber phantom generics, including sub event
//  - SubscriptionIdentifier - find way to do ref in same impl maybe with Cow? AsRef etc?
//    '--> Can it be same as Identifier w/ some magic deref craziness?
//  - ExchangeSubscription to ExchangeSubMeta? Doesn't seem to really fit since it's not 1-to-1 with ExchangeEvent
//  - Go through and add derives #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
//  - Go through and select appropriate access modifiers for everything
//  - Go through and add tests from develop branch for each part that I've left out (fine tooth comb)
//  - SubscriptionId should probably contain a reference to a String... then use serde borrow
//  - Coinbase Pro has some initial snapshot that's coming through after sub validation succeeds...?
//  - Add TradeId new type to barter-integration, etc.
//  - normalise module structure. ie/ use domain consistently
//  - Should I hard-code the idea of a SubMeta? It can be market & channel always. Then enforce Subscription<Kind>: Identifier<Channel>

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Exchange: Transformer> = ExchangeStream<
    WebSocketParser, WsStream, Exchange, Exchange::Output
>;

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

impl<Exchange, Kind, ExchangeEvent> Transformer for ExchangeTransformer<Exchange, Kind, ExchangeEvent>
where
    Exchange: ExchangeMeta<ExchangeEvent>,
    Kind: SubKind,
    ExchangeEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, ExchangeEvent)>,
{
    type Input = ExchangeEvent;
    type Output = Market<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, event: Self::Input) -> Self::OutputIter {
        // Find Instrument associated with Input and transform
        match self.subscription_map.find_instrument(&event.id()) {
            Ok(instrument) => {
                MarketIter::<Kind::Event>::from((Exchange::exchange_id(), instrument, event)).0
            },
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            },
        }
    }
}

impl<Exchange, Kind, ExchangeEvent> ExchangeTransformer<Exchange, Kind, ExchangeEvent> {
    pub fn new(subscription_map: SubscriptionMap<Kind>) -> Self {
        Self {
            subscription_map,
            phantom: PhantomData::<(Exchange, ExchangeEvent)>::default()
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
