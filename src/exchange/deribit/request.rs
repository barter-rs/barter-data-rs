use crate::exchange::deribit::channel::DeribitChannel;
use crate::exchange::deribit::market::DeribitMarket;
use crate::exchange::subscription::ExchangeSub;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitRequest<'a, T> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
    params: T,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitSubParams {
    channels: Vec<String>,
}

impl FromIterator<ExchangeSub<DeribitChannel<'static>, DeribitMarket>>
    for DeribitRequest<'_, DeribitSubParams>
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = ExchangeSub<DeribitChannel<'static>, DeribitMarket>>,
    {
        Self {
            jsonrpc: "2.0",
            id: 0,
            method: "public/subscribe",
            params: DeribitSubParams {
                channels: iter
                    .into_iter()
                    .map(|ExchangeSub { channel, market }| {
                        format!("{}.{}.agg2", channel.as_ref(), market.as_ref())
                    })
                    .collect::<Vec<_>>(),
            },
        }
    }
}
