use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use tokio::sync::mpsc;
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::{ExchangeId, ExchangeTransformer};
use crate::exchange::binance::model::BinanceTrade;
use crate::model::{Market, PublicTrade};
use crate::model::subscription::{SubKind, SubscriptionMap};

impl ExchangeTransformer for BinanceFuturesUsd<BinanceTrade> {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn new(_: mpsc::UnboundedSender<WsMessage>, subscription_map: SubscriptionMap<BinanceTrade>) -> Self {
        Self { subscription_map }
    }
}

impl Transformer for BinanceFuturesUsd<BinanceTrade> {
    type Input = BinanceTrade;
    type Output = Market<PublicTrade>;
    type OutputIter = Vec<Result<Market<PublicTrade>, SocketError>>;

    fn transform(&mut self, trade: Self::Input) -> Self::OutputIter {
        match self.subscription_map.find_instrument(&trade.subscription_id) {
            Ok(instrument) => {
                vec![Ok(Self::build_market_event(instrument, trade))]
            }
            Err(error) => {
                vec![Err(error)]
            }
        }
    }
}

