use super::{BinanceSubResponse, BinanceMessage};
use crate::{
    ExchangeTransformerId, Subscriber, ExchangeTransformer, Subscription, SubscriptionMeta, SubscriptionIds,
    error::DataError,
    model::{StreamKind, MarketData}
};
use barter_integration::socket::{Transformer, error::SocketError};
use serde::{Deserialize, Serialize};


#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFutures {
    pub ids: SubscriptionIds,
}

impl Subscriber for BinanceFutures {
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str { "wss://fstream.binance.com/ws" }

    fn build_subscription_meta(subscriptions: &[Subscription]) -> Result<SubscriptionMeta, DataError> {
        todo!()
    }
}

impl ExchangeTransformer for BinanceFutures {
    const EXCHANGE: ExchangeTransformerId = ExchangeTransformerId::BinanceFutures;

    fn new(ids: SubscriptionIds) -> Self { Self { ids } }
}

impl Transformer<MarketData> for BinanceFutures {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketData, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let market_data = match input {
            BinanceMessage::Trade(trade) => {
                    MarketData::from((
                        BinanceFutures::EXCHANGE, self.ids.find_instrument(&trade)?, trade
                    ))
            },
        };

        vec![Ok(market_data)]
    }
}

impl BinanceFutures {
    fn get_channel_id(sub: &Subscription) -> Result<String, DataError> {
        match &sub.kind {
            StreamKind::Trades => Ok(format!("{}{}@aggTrade", sub.instrument.base, sub.instrument.quote)),
            other =>  Err(DataError::Unsupported {
                entity: BinanceFutures::EXCHANGE.as_str(),
                item: other.to_string()
            })
        }
    }
}