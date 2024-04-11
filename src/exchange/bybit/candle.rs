use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::message::BybitPayload, ExchangeId},
    subscription::candle::Candle,
};
use barter_integration::model::{instrument::Instrument, Exchange};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`BybitCandle`](BybitCandleInner) candles WebSocket message.
pub type BybitCandle = BybitPayload<Vec<BybitCandleInner>>;

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/kline>
/// Candle
///```json
/// {
///     "start": 1672324800000,
//      "end": 1672325099999,
//      "interval": "5",
//      "open": "16649.5",
//      "close": "16677",
//      "high": "16677",
//      "low": "16608",
//      "volume": "2.081",
//      "turnover": "34666.4005",
//      "confirm": false,
//      "timestamp": 1672324988882
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitCandleInner {
    #[serde(deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub start: DateTime<Utc>,

    #[serde(deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub end: DateTime<Utc>,

    #[serde(alias = "interval")]
    pub period: String,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub open: f64,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub close: f64,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub high: f64,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub low: f64,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub volume: f64,

    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub turnover: f64,

    pub confirm: bool,

    #[serde(deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub timestamp: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, BybitCandle)> for MarketIter<Candle> {
    fn from((exchange_id, instrument, candles): (ExchangeId, Instrument, BybitCandle)) -> Self {
        Self(
            candles
                .data
                .into_iter()
                .filter(|candle| candle.confirm)
                .map(|candle| {
                    Ok(MarketEvent {
                        exchange_time: candle.timestamp,
                        received_time: Utc::now(),
                        exchange: Exchange::from(exchange_id),
                        instrument: instrument.clone(),
                        kind: Candle {
                            start_time: candle.start,
                            open: candle.open,
                            high: candle.high,
                            low: candle.low,
                            close: candle.close,
                            volume: candle.volume,
                        },
                    })
                })
                .collect(),
        )
    }
}
