use super::{
    BinanceFuturesUsd,
    super::model::BinanceMessage,
};
use crate::{
    model::MarketEvent,
    ExchangeId, ExchangeTransformer, SubscriptionMap,
};
use barter_integration::{
    error::SocketError, protocol::websocket::WsMessage, Transformer,
};
use tokio::sync::mpsc;

impl ExchangeTransformer for BinanceFuturesUsd {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;
    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionMap) -> Self {
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
                        trade,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
            BinanceMessage::OrderBookSnapshot(snapshot) => {
                match self.ids.find_instrument(&snapshot.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        snapshot,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
            BinanceMessage::Liquidation(liquidation) => {
                match self.ids.find_instrument(&liquidation.order.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        liquidation,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
        }
    }
}