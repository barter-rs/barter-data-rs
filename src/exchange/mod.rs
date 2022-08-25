use chrono::{DateTime, Utc};
use serde::de;
use std::{str::FromStr, time::Duration};

/// Binance `ExchangeTransformer` & `Subscriber` implementations.
pub mod binance;

/// Ftx `ExchangeTransformer` & `Subscriber` implementations.
pub mod ftx;

/// Deserialize a `String` as the desired type.
pub fn de_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let data: String = de::Deserialize::deserialize(deserializer)?;
    data.parse::<T>().map_err(de::Error::custom)
}

/// Determine the `DateTime<Utc>` from the provided `Duration` since the epoch.
pub fn datetime_utc_from_duration(duration: Duration) -> DateTime<Utc> {
    DateTime::<Utc>::from(std::time::UNIX_EPOCH) + duration
}
