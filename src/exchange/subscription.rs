use crate::{subscription::Subscription, Identifier};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct ExchangeSub<Channel, Market> {
    pub channel: Channel,
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
    /// Todo:
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
