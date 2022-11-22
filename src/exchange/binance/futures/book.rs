use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use tokio::sync::mpsc;
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::{ExchangeId, ExchangeTransformer};
use crate::exchange::binance::model::{BinanceOrderBook, BinanceTrade};
use crate::model::{Market, OrderBook};
use crate::model::subscription::{SubKind, SubscriptionMap};

impl ExchangeTransformer<BinanceOrderBook> for BinanceFuturesUsd<BinanceOrderBook> {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn new(_: mpsc::UnboundedSender<WsMessage>, subscription_map: SubscriptionMap<BinanceOrderBook>) -> Self {
        Self { subscription_map }
    }
}

impl Transformer<Market<<BinanceOrderBook as SubKind>::Event>> for BinanceFuturesUsd<BinanceOrderBook> {
    type Input = BinanceOrderBook;
    type OutputIter = Vec<Result<Market<OrderBook>, SocketError>>;

    fn transform(&mut self, snapshot: Self::Input) -> Self::OutputIter {
        match self.subscription_map.find_instrument(&snapshot.subscription_id) {
            Ok(instrument) => {
                vec![Ok(Self::build_market_event(instrument, snapshot))]
            },
            Err(error) => vec![Err(error)],
        }
    }
}