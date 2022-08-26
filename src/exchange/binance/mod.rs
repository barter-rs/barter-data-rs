use super::{datetime_utc_from_epoch_duration, de_str};
use crate::{
    model::{DataKind, PublicTrade},
    ExchangeId, Identifiable, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// `BinanceFuturesUsd` specific [`Subscriber`](crate::Subscriber) &
/// [`ExchangeTransformer`](crate::ExchangeTransformer) implementor for the collection of
/// Futures data.
pub mod futures;

/// `Binance` & `BinanceFuturesUsd` `Subscription` response message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#live-subscribing-unsubscribing-to-streams>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}

/// `Binance` message variants that could be received over [`WebSocket`](crate::WebSocket).
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum BinanceMessage {
    Trade(BinanceTrade),
}

impl Identifiable for BinanceMessage {
    fn id(&self) -> SubscriptionId {
        match self {
            BinanceMessage::Trade(trade) => SubscriptionId::from(trade),
        }
    }
}

impl From<(ExchangeId, Instrument, BinanceMessage)> for MarketEvent {
    fn from((exchange, instrument, message): (ExchangeId, Instrument, BinanceMessage)) -> Self {
        match message {
            BinanceMessage::Trade(trade) => MarketEvent::from((exchange, instrument, trade)),
        }
    }
}

/// `Binance` real-time trade message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "T")]
    trade_ts: u64,
    #[serde(rename = "a")]
    id: u64,
    #[serde(rename = "p", deserialize_with = "de_str")]
    price: f64,
    #[serde(rename = "q", deserialize_with = "de_str")]
    quantity: f64,
    #[serde(rename = "m")]
    buyer_is_maker: bool,
}

impl From<&BinanceTrade> for SubscriptionId {
    fn from(trade: &BinanceTrade) -> Self {
        SubscriptionId(format!(
            "{}@{}",
            trade.symbol.to_lowercase(),
            trade.event_type
        ))
    }
}

impl From<(ExchangeId, Instrument, BinanceTrade)> for MarketEvent {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, BinanceTrade)) -> Self {
        Self {
            exchange_time: datetime_utc_from_epoch_duration(Duration::from_millis(trade.trade_ts)),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.quantity,
                side: if trade.buyer_is_maker {
                    Side::Sell
                } else {
                    Side::Buy
                },
            }),
        }
    }
}
