use super::Binance;
use crate::exchange::{ExchangeId, ServerSelector};
use serde::{Deserialize, Serialize};

/// [`BinanceSpot`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
pub const BASE_URL_BINANCE_SPOT: &str = "wss://stream.binance.com:9443/ws";

/// Todo:
pub type BinanceSpot = Binance<BinanceServerSpot>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct BinanceServerSpot;

impl ServerSelector for BinanceServerSpot {
    const ID: ExchangeId = ExchangeId::BinanceSpot;

    fn base_url() -> &'static str {
        BASE_URL_BINANCE_SPOT
    }
}
