use chrono::{DateTime, Utc};
use serde::ser::SerializeSeq;
use serde::{de, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{str::FromStr, time::Duration};

/// Binance `ExchangeTransformer` & `Subscriber` implementations.
pub mod binance;

/// Ftx `ExchangeTransformer` & `Subscriber` implementations.
pub mod ftx;

/// Kraken `ExchangeTransformer` & `Subscriber` implementations.
pub mod kraken;

/// Coinbase `ExchangeTransformer` & `Subscriber` implementations.
pub mod coinbase;

/// Kucoin ''ExchangeTransformer' & 'Subscriber' implementations.
pub mod kucoin;

/// Determine the `DateTime<Utc>` from the provided `Duration` since the epoch.
pub fn datetime_utc_from_epoch_duration(duration: Duration) -> DateTime<Utc> {
    DateTime::<Utc>::from(std::time::UNIX_EPOCH + duration)
}

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

/// Deserialize a `u64` as `DateTime<Utc>`.
pub fn de_u64_epoch_ms_as_datetime_utc<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let epoch_ms: u64 = de::Deserialize::deserialize(deserializer)?;
    Ok(datetime_utc_from_epoch_duration(Duration::from_millis(
        epoch_ms,
    )))
}

/// Assists deserialisation of sequences by attempting to extract & parse the next element in the
/// provided sequence.
///
/// A [`serde::de::Error`] is returned if the element does not exist, or it cannot
/// be deserialized into the `Target` type inferred.
///
/// Example sequence: ["20180.30000","0.00010000","1661978265.280067","s","l",""]
pub fn extract_next<'de, SeqAccessor, Target>(
    sequence: &mut SeqAccessor,
    name: &'static str,
) -> Result<Target, SeqAccessor::Error>
where
    SeqAccessor: de::SeqAccess<'de>,
    Target: de::DeserializeOwned,
{
    sequence
        .next_element::<Target>()?
        .ok_or_else(|| de::Error::missing_field(name))
}

/// Serialize a generic element T as a `Vec<T>`.
pub fn se_element_to_vector<T, S>(element: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Serialize,
{
    let mut sequence = serializer.serialize_seq(Some(1))?;
    sequence.serialize_element(&element)?;
    sequence.end()
}

/// Get the UNIX timestamp in ms. Useful for id'ing messages.
pub fn get_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_millis()
}
