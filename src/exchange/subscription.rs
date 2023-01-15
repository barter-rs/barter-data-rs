use crate::{subscription::Subscription, Identifier};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

/// Defines an exchange specific market and channel combination used by an exchange [`Connector`] to
/// build the [`WsMessage`] subscription payloads to send to the exchange server.
///
/// ### Examples
/// #### Binance OrderBooksL2
/// ```json
/// ExchangeSub {
///     channel: BinanceChannel("@depth@100ms"),
///     market: BinanceMarket("btcusdt"),
/// }
/// ```
/// #### Kraken PublicTrades
/// ```json
/// ExchangeSub {
///     channel: KrakenChannel("trade"),
///     market: KrakenChannel("BTC/USDT")
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct ExchangeSub<Channel, Market> {
    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// channel to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](BinanceChannel)
    /// - [`KrakenChannel("trade")`](KrakenChannel)
    pub channel: Channel,

    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// market that can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](KrakenMarket)
    pub market: Market,
}

impl<Channel, Market> Identifier<SubscriptionId> for ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn id(&self) -> SubscriptionId {
        SubscriptionId::from(format!(
            "{}|{}",
            self.channel.as_ref(),
            self.market.as_ref()
        ))
    }
}

impl<Channel, Market> ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    /// Construct a new exchange specific [`Self`] with the Barter [`Subscription`] provided.
    pub fn new<Exchange, Kind>(sub: &Subscription<Exchange, Kind>) -> Self
    where
        Subscription<Exchange, Kind>: Identifier<Channel> + Identifier<Market>,
    {
        Self {
            channel: sub.id(),
            market: sub.id(),
        }
    }
}

impl<Channel, Market> From<(Channel, Market)> for ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn from((channel, market): (Channel, Market)) -> Self {
        Self { channel, market }
    }
}
