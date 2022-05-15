use crate::{ExchangeTransformer, ExchangeTransformerId, MarketData, Subscriber, exchange::ftx::model::{FtxSubResponse, FtxMessage}, Identifiable};
use barter_integration::{
    StreamKind, Subscription, InstrumentKind, SubscriptionId, SubscriptionIds, SubscriptionMeta,
    socket::{
        Transformer,
        error::SocketError,
        protocol::websocket::WsMessage,
    }
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

mod model;

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Ftx {
    pub ids: SubscriptionIds,
}

impl Subscriber for Ftx {
    type SubResponse = FtxSubResponse;

    fn base_url() -> &'static str { "wss://ftx.com/ws/" }

    fn build_subscription_meta(subscriptions: &[Subscription]) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to Ftx channels
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the Ftx specific channel & market for this Barter Subscription
                let (channel, market) = Self::get_channel_meta(subscription)?;

                // Construct Ftx specific subscription message
                let ftx_subscription = Self::subscription(channel, &market);

                // Use market as the SubscriptionId key in the SubscriptionIds
                ids.0.insert(SubscriptionId(market), subscription.clone());

                Ok(ftx_subscription)
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for Ftx {
    const EXCHANGE: ExchangeTransformerId = ExchangeTransformerId::Ftx;
    fn new(ids: SubscriptionIds) -> Self { Self { ids } }
}

impl Transformer<MarketData> for Ftx {
    type Input = FtxMessage;
    type OutputIter = Vec<Result<MarketData, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let instrument = match self.ids.find_instrument(input.id()) {
            Ok(instrument) => instrument,
            Err(error) => return vec![Err(error)]
        };

        match input {
            FtxMessage::Trades { trades, .. } => {
                trades
                    .into_iter()
                    .map(|trade| Ok(MarketData::from(
                        (Ftx::EXCHANGE, instrument.clone(), trade)
                    )))
                    .collect()
            }
        }
    }
}

impl Ftx {
    /// Determine the `Ftx` channel metadata associated with an input Barter [`Subscription`].
    /// This includes the `Ftx` &str channel, and a `String` market identifier. Both are used to
    /// build an `Ftx` subscription payload.
    ///
    /// Example Ok Return: Ok("trades", "BTC/USDT")
    /// where channel == "trades" & market == "BTC/USDT".
    fn get_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Determine Ftx channel using the Subscription StreamKind
        let channel = match &sub.kind {
            StreamKind::Trade => "trades",
            other => return Err(SocketError::Unsupported {
                entity: Self::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        };

        // Determine Ftx market using the InstrumentKind
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!("{}/{}", sub.instrument.base, sub.instrument.quote).to_uppercase(),
            InstrumentKind::FuturePerpetual => format!("{}-PERP", sub.instrument.base).to_uppercase(),
        };

        Ok((channel, market))
    }

    /// Build a `Ftx` compatible subscription message using the channel & market provided.
    fn subscription(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "op": "subscribe",
                "channel": channel,
                "market": market,
            }).to_string(),
        )
    }
}