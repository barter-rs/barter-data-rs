/// Binance `ExchangeTransformer` & `Subscriber` implementations.
pub mod binance;

/// Ftx `ExchangeTransformer` & `Subscriber` implementations.
pub mod ftx;

use std::{
    time::Duration,
    str::FromStr
};
use serde::de;
use chrono::{DateTime, Utc};

/// Deserialize a string as the desired type.
pub fn de_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let data: String = de::Deserialize::deserialize(deserializer)?;
    data.parse::<T>().map_err(de::Error::custom)
}

/// Determine the `DateTime<Utc>` from the provided `u64` milliseconds since the epoch.
pub fn epoch_ms_to_datetime_utc(epoch_ms: u64) -> DateTime<Utc> {
    DateTime::<Utc>::from(std::time::UNIX_EPOCH + Duration::from_millis(epoch_ms))
}