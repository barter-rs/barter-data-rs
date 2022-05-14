use super::{BinanceSubResponse, BinanceMessage};
use crate::{
    ExchangeTransformerId, Subscriber, ExchangeTransformer, MarketEvent, Subscription,
    SubscriptionMeta, SubscriptionIds,
    error::DataError,
    model::StreamKind
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

impl Transformer<MarketEvent> for BinanceFutures {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        todo!()
        //
        // match input {
        //     BinanceMessage::Trade(_) => {}
        // }
        //
        // // Output vector to return (only ever 0 or 1 length)
        // let mut output_iter = Vec::with_capacity(1);
        //
        // match input {
        //     BinanceMessage::Trade(trade) => {
        //         let (instrument, sequence) = match self.get_stream_meta(&trade.to_stream_id()) {
        //             Ok(stream_meta) => stream_meta,
        //             Err(err) => {
        //                 output_iter.push(Err(err));
        //                 return output_iter;
        //             },
        //         };
        //
        //         output_iter.push(Ok(MarketEvent::new(
        //             sequence,
        //             MarketData::from((BinanceFutures::EXCHANGE, instrument, trade))
        //         )))
        //     }
        // };
        //
        // output_iter
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