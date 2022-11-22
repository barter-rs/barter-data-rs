use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use tokio::sync::mpsc;
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::exchange::binance::model::BinanceLiquidation;
use crate::{ExchangeId, ExchangeTransformer};
use crate::model::{Liquidation, Market};
use crate::model::subscription::{SubKind, SubscriptionMap};

impl ExchangeTransformer<BinanceLiquidation> for BinanceFuturesUsd<BinanceLiquidation> {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn new(_: mpsc::UnboundedSender<WsMessage>, subscription_map: SubscriptionMap<BinanceLiquidation>) -> Self {
        Self { subscription_map }
    }
}

impl Transformer<Market<<BinanceLiquidation as SubKind>::Event>> for BinanceFuturesUsd<BinanceLiquidation> {
    type Input = BinanceLiquidation;
    type OutputIter = Vec<Result<Market<Liquidation>, SocketError>>;

    fn transform(&mut self, liquidation: Self::Input) -> Self::OutputIter {
        match self.subscription_map.find_instrument(&liquidation.order.subscription_id) {
            Ok(instrument) => {
                vec![Ok(Self::build_market_event(instrument, liquidation))]
            },
            Err(error) => {
                vec![Err(error)]
            },
        }
    }
}