use crate::{
    ExchangeTransformerId, StreamId, StreamIdentifier,
    model::{Direction, MarketData, Trade}
};
use barter_integration::{
    Instrument,
    util::epoch_ms_to_datetime_utc
};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use rust_decimal::Decimal;

pub mod futures;

// Todo: Add proper error enum for BinanceMessage in Barter-Data
//     '--> eg/ enum BinanceMessage { Error, BinancePayload }

/// Binance Message variants that could be received over [`WebSocket`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum BinanceMessage {
    Subscribed(BinanceSubscribed),
    Trade(BinanceTrade)
}

/// Binance specific subscription confirmation message.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubscribed {
    result: Option<Vec<String>>,
    id: u32,
}

impl BinanceSubscribed {
    pub fn is_failure(&self) -> bool {
        self.result.is_some()
    }
}

/// Binance specific Trade message.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "T")]
    trade_ts: u64,
    #[serde(rename = "a")]
    id: u64,
    #[serde(rename = "p")]
    price: Decimal,
    #[serde(rename = "q")]
    quantity: Decimal,
    #[serde(rename = "m")]
    buyer_is_maker: bool,
}

impl StreamIdentifier for BinanceTrade {
    fn to_stream_id(&self) -> StreamId {
        format!("{}@{}", self.symbol.to_lowercase(), self.event_type).into()
    }
}

impl From<(ExchangeTransformerId, Instrument, BinanceTrade)> for MarketData {
    fn from((exchange, instrument, trade): (ExchangeTransformerId, Instrument, BinanceTrade)) -> Self {
        Self::Trade(Trade {
            id: trade.id.to_string(),
            exchange: exchange.to_string(),
            instrument,
            received_timestamp: Utc::now(),
            exchange_timestamp: epoch_ms_to_datetime_utc(trade.trade_ts),
            price: trade.price,
            quantity: trade.quantity,
            direction: if trade.buyer_is_maker {
                Direction::Sell
            } else {
                Direction::Buy
            }
        })
    }
}