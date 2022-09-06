use super::model::{BinanceMessage, BinanceSubResponse};
use crate::{
    model::{MarketEvent, SubKind},
    ExchangeId, ExchangeTransformer, Subscriber, Subscription, SubscriptionIds, SubscriptionMeta,
};
use barter_integration::{error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer, Validator};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// `BinanceFuturesUsd` [`Subscriber`](crate::Subscriber) &
/// [`ExchangeTransformer`](crate::ExchangeTransformer) implementor for the collection
/// of `Futures` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesUsd {
    pub ids: SubscriptionIds,
}

impl Subscriber for BinanceFuturesUsd {
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str {
        "wss://fstream.binance.com/ws"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to BinanceFuturesUsd 'StreamNames'
        let stream_names = subscriptions
            .iter()
            .map(|subscription| {
                // Determine BinanceFuturesUsd specific channel & market for this Barter Subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the SubscriptionId key in the SubscriptionIds
                // eg/ SubscriptionId("@depth@100ms|BTCUSDT")
                ids.insert(BinanceFuturesUsd::subscription_id(channel, &market), subscription.clone());

                // Construct BinanceFuturesUsd 'StreamName' eg/ "btcusdt@aggTrade"
                Ok(format!("{market}{channel}"))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        // Use channels to construct a Binance subscription WsMessage
        let subscriptions = Self::subscriptions(stream_names);

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for BinanceFuturesUsd {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;
    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for BinanceFuturesUsd {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            BinanceMessage::Trade(trade) => {
                match self.ids.find_instrument(&trade.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        trade
                    )))],
                    Err(error) => vec![Err(error)]
                }
            }
            BinanceMessage::OrderBookL2Update(update) => {
                match self.ids.find_instrument(&update.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        update
                    )))],
                    Err(error) => vec![Err(error)]
                }
            }
        }
    }
}

impl BinanceFuturesUsd {
    /// [`BinanceFuturesUsd`] aggregated trades channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#aggregate-trade-streams>
    pub const CHANNEL_TRADES: &'static str = "@aggTrade";

    /// [`BinanceFuturesUsd`] L2 OrderBook channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#diff-book-depth-streams>
    pub const CHANNEL_ORDER_BOOK_L2: &'static str = "@depth@100ms";

    /// Determine the [`BinanceFuturesUsd`] channel metadata associated with an input
    /// Barter [`Subscription`]. This includes the [`BinanceFuturesUsd`] `&str` channel
    /// identifier, and a `String` market identifier. Both are used to build a
    /// [`BinanceFuturesUsd`] subscription payload.
    ///
    /// Example Ok return: Ok("@aggTrade", "btcusdt")
    /// where channel == "@aggTrade" & market == "btcusdt"
    pub fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Validate provided Subscription InstrumentKind is supported by BinanceFuturesUsd
        let sub = sub.validate()?;

        // Determine the BinanceFuturesUsd channel
        let channel = match &sub.kind {
            SubKind::Trade => Self::CHANNEL_TRADES,
            SubKind::OrderBookL2 => Self::CHANNEL_ORDER_BOOK_L2,
            other => return Err(SocketError::Unsupported {
                    entity: BinanceFuturesUsd::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
        };

        // Determine BinanceFuturesUsd market using the Instrument
        let market = format!("{}{}", sub.instrument.base, sub.instrument.quote);//.to_uppercase(); // Todo: to upper works?

        Ok((channel, market))
    }

    /// Build a [`BinanceFuturesUsd`] compatible [`SubscriptionId`] using the channel & market
    /// provided. This is used to associate [`BinanceFuturesUsd`] data structures received over
    /// the WebSocket with it's original Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("@depth@100ms|BTCUSDT")
    pub fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }

    /// Build a [`BinanceFuturesUsd`] compatible subscription message using the
    /// 'StreamNames' provided.
    pub fn subscriptions(stream_names: Vec<String>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }
}
