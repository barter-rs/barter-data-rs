use super::SubKind;
use crate::model::Candle;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Candles(Interval);

impl SubKind for Candles {
    type Event = Candle;
}

/// Barter time interval used for specifying the interval of a [`Candles`] [`SubKind`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Interval {
    #[serde(alias = "1m")]
    Minute1,
    #[serde(alias = "3m")]
    Minute3,
    #[serde(alias = "5m")]
    Minute5,
    #[serde(alias = "15m")]
    Minute15,
    #[serde(alias = "30m")]
    Minute30,
    #[serde(alias = "1h")]
    Hour1,
    #[serde(alias = "2h")]
    Hour2,
    #[serde(alias = "4h")]
    Hour4,
    #[serde(alias = "6h")]
    Hour6,
    #[serde(alias = "8h")]
    Hour8,
    #[serde(alias = "12h")]
    Hour12,
    #[serde(alias = "1d")]
    Day1,
    #[serde(alias = "3d")]
    Day3,
    #[serde(alias = "1w")]
    Week1,
    #[serde(alias = "1M")]
    Month1,
    #[serde(alias = "3M")]
    Month3,
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Interval::Minute1 => "1m",
                Interval::Minute3 => "3m",
                Interval::Minute5 => "5m",
                Interval::Minute15 => "15m",
                Interval::Minute30 => "30m",
                Interval::Hour1 => "1h",
                Interval::Hour2 => "2h",
                Interval::Hour4 => "4h",
                Interval::Hour6 => "6h",
                Interval::Hour8 => "8h",
                Interval::Hour12 => "12h",
                Interval::Day1 => "1d",
                Interval::Day3 => "3d",
                Interval::Week1 => "1w",
                Interval::Month1 => "1M",
                Interval::Month3 => "3M",
            }
        )
    }
}
