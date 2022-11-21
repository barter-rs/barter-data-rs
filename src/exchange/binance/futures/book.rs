// use std::marker::PhantomData;
// use barter_integration::error::SocketError;
// use barter_integration::model::Instrument;
// use barter_integration::protocol::http::HttpParser;
// use barter_integration::protocol::http::public::PublicNoHeaders;
// use barter_integration::protocol::http::rest::client::RestClient;
// use barter_integration::protocol::websocket::WsMessage;
// use barter_integration::Transformer;
// use futures::Stream;
// use tokio::sync::mpsc;
// use tokio::sync::mpsc::UnboundedSender;
// use crate::exchange::binance::model::BinanceMessage;
// use crate::{ExchangeId};
// use crate::model::{MarketEvent, PublicTrade};
// use crate::model::subscription::SubscriptionMap;
//
// struct BinanceHttpParser;
// impl HttpParser for BinanceHttpParser {
//     type ApiError = ();
//     type OutputError = SocketError;
//     fn parse_api_error(&self, status: reqwest::StatusCode, error: Self::ApiError) -> Self::OutputError {
//         todo!()
//     }
// }
// struct OrderBook;
// struct Trade { id: String }
//
// struct SubscriptionNew<Kind> {
//     exchange: ExchangeId,
//     instrument: Instrument,
//     kind: Kind,
// }
//
// trait SubKindNew {
//     type Output;
// }
//
// impl SubKindNew for Trade {
//     type Output = Self;
// }
//
// trait MarketStreamNew<Kind: SubKindNew>: Stream<Item = Result<Kind::Output, SocketError>> + Sized + Unpin {
//     async fn init(subscriptions: &[SubscriptionNew<Kind>]) -> Result<Self, SocketError>;
// }
//
// trait ExchangeTransformer<T>: Transformer<T> + Sized
// where
//     T: Into<MarketEvent>
// {
//     const EXCHANGE: ExchangeId;
//     fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionMap) -> Self;
// }
//
// struct BinanceFuturesUsd<Transform, T>
// where
//     Transform: Transformer<T>,
//     T: Into<MarketEvent>
// {
//     pub ids: SubscriptionMap,
//     pub transformer: Transform,
//     pub phantom: PhantomData<T>,
// }
//
// impl Transformer<OrderBook> for BinanceFuturesUsd<BinanceBookTransformer, OrderBook> {
//     type Input = ();
//     type OutputIter = ();
//
//     fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
//         todo!()
//     }
// }
//
// impl ExchangeTransformer<OrderBook> for BinanceFuturesUsd<BinanceBookTransformer, OrderBook> {
//     const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;
//
//     fn new(ws_sink_tx: UnboundedSender<WsMessage>, ids: SubscriptionMap) -> Self {
//         todo!()
//     }
// }
//
// struct BinanceBookTransformer<'a> {
//     pub ids: &'a SubscriptionMap,
//     pub rest_client: RestClient<'a, PublicNoHeaders, BinanceHttpParser>,
// }
// struct BinanceTradeTransformer<'a> {
//     pub ids: &'a SubscriptionMap,
// }
// impl Transformer<OrderBook> for BinanceBookTransformer<'_> {
//     type Input = BinanceMessage;
//     type OutputIter = Vec<Result<OrderBook, SocketError>>;
//
//     fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
//         todo!()
//     }
// }
// impl Transformer<Trade> for BinanceTradeTransformer<'_> {
//     type Input = BinanceMessage;
//     type OutputIter = Vec<Result<Trade, SocketError>>;
//
//     fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
//         todo!()
//     }
// }
// impl From<Trade> for MarketEvent {
//     fn from(_: Trade) -> Self {
//         todo!()
//     }
// }
// impl From<OrderBook> for MarketEvent {
//     fn from(_: OrderBook) -> Self {
//         todo!()
//     }
// }