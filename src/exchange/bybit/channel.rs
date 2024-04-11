use crate::{
    exchange::bybit::Bybit,
    subscription::{
        candle::{CandlePeriod, Candles},
        trade::PublicTrades,
        Subscription,
    },
    Identifier,
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`](super::Bybit)
/// channel to be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BybitChannel(pub &'static str);

impl BybitChannel {
    /// [`Bybit`](super::Bybit) real-time trades channel name.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
    pub const TRADES: Self = Self("publicTrade");

    /// [`Bybit`](super::Bybit) real-time candles channel names.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/kline>
    pub const CANDLES_1M: Self = Self("kline.1");
    pub const CANDLES_3M: Self = Self("kline.3");
    pub const CANDLES_5M: Self = Self("kline.5");
    pub const CANDLES_15M: Self = Self("kline.15");
    pub const CANDLES_30M: Self = Self("kline.30");
    pub const CANDLES_1H: Self = Self("kline.60");
    pub const CANDLES_2H: Self = Self("kline.120");
    pub const CANDLES_4H: Self = Self("kline.240");
    pub const CANDLES_6H: Self = Self("kline.360");
    pub const CANDLES_12H: Self = Self("kline.720");
    pub const CANDLES_1D: Self = Self("kline.D");
    pub const CANDLES_1W: Self = Self("kline.W");
    pub const CANDLES_1MTH: Self = Self("kline.M");
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, PublicTrades> {
    fn id(&self) -> BybitChannel {
        BybitChannel::TRADES
    }
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, Candles> {
    fn id(&self) -> BybitChannel {
        match self.kind.0 {
            CandlePeriod::OneMinute => BybitChannel::CANDLES_1M,
            CandlePeriod::ThreeMinutes => BybitChannel::CANDLES_3M,
            CandlePeriod::FiveMinutes => BybitChannel::CANDLES_5M,
            CandlePeriod::FifteenMinutes => BybitChannel::CANDLES_15M,
            CandlePeriod::ThirtyMinutes => BybitChannel::CANDLES_30M,
            CandlePeriod::OneHour => BybitChannel::CANDLES_1H,
            CandlePeriod::TwoHours => BybitChannel::CANDLES_2H,
            CandlePeriod::FourHours => BybitChannel::CANDLES_4H,
            CandlePeriod::SixHours => BybitChannel::CANDLES_6H,
            CandlePeriod::TwelveHours => BybitChannel::CANDLES_12H,
            CandlePeriod::OneDay => BybitChannel::CANDLES_1D,
            CandlePeriod::OneWeek => BybitChannel::CANDLES_1W,
            CandlePeriod::OneMonth => BybitChannel::CANDLES_1MTH,
        }
    }
}

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
