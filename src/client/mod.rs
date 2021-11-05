use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Binance WebSocket client implementing the ExchangeClient trait.
pub mod binance;

/// Bitstamp WebSocket client implementing the ExchangeClient trait.
pub mod bitstamp;

/// Common client configuration that is likely required by an ExchangeClient trait implementor.
#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    pub rate_limit_per_minute: u64,
}

/// Possible exchange Client names.
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub enum ClientName {
    Binance,
}

impl Display for ClientName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ClientName::Binance => "Binance",
        };
        write!(f, "{}", name)
    }
}

/// Custom [Deserializer] function to deserialize an input [str] to a [f64].
pub fn de_str_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let input_str: &str = Deserialize::deserialize(deserializer)?;
    f64::from_str(input_str).map_err(de::Error::custom)
}
