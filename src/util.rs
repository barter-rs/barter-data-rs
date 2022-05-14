use std::time::Duration;
use chrono::{DateTime, Utc};

/// Determine the `DateTime<Utc>` from the provided `u64` milliseconds since the epoch.
pub fn epoch_ms_to_datetime_utc(epoch_ms: u64) -> DateTime<Utc> {
    DateTime::<Utc>::from(std::time::UNIX_EPOCH + Duration::from_millis(epoch_ms))
}