use crate::client::{de_str_to_f64, ClientConfig};
use crate::connection::ConnectionHandler;
use crate::error::ClientError;
use crate::model::BuyerType;
use crate::{connect, Candle, ExchangeClient, Identifier, StreamIdentifier, Subscription, Trade};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// [ExchangeClient] implementation for Binance.
pub struct Binance {
    /// [Subscription] request channel transmitter. Transmits a tuple of [Subscription] and a data
    /// channel transmitter. This data channel transmitter is used to send messages relating the
    /// [Subscription] back to this Binance client from the [ConnectionHandler].
    subscription_tx: mpsc::Sender<(BinanceSub, mpsc::UnboundedSender<BinanceMessage>)>,
}

#[async_trait]
impl ExchangeClient for Binance {
    async fn consume_trades(
        &mut self,
        symbol: String,
    ) -> Result<UnboundedReceiverStream<Trade>, ClientError> {
        // Construct trades channel that ConnectionHandler will distribute trade stream data on
        let (binance_trade_tx, mut binance_trade_rx) = mpsc::unbounded_channel();

        // Construct Subscription for the ConnectionHandler to action
        let trades_subscription = BinanceSub::new(String::from(Binance::TRADE_STREAM), symbol);

        // Subscribe by passing a tuple of (Subscription, trade_tx) to the ConnectionHandler
        if let Err(err) = self
            .subscription_tx
            .send((trades_subscription, binance_trade_tx))
            .await
        {
            error!("Subscription request receiver has dropped by the ConnectionHandler - closing transmitter: {:?}", err);
            return Err(ClientError::SendFailure);
        }

        // Construct channel to distribute normalised Trade data to downstream consumers
        let (trade_tx, trade_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(binance_message) = binance_trade_rx.recv().await {
                match binance_message {
                    BinanceMessage::Trade(binance_trade) => {
                        if trade_tx.send(Trade::from(binance_trade)).is_err() {
                            info!("Receiver for Binance Trades has been dropped - closing stream.");
                            return;
                        }
                    }
                    _ => warn!("consume_trades() received BinanceMessage that was not a Trade"),
                }
            }
        });

        // Return normalised Trade stream to consumer
        Ok(UnboundedReceiverStream::new(trade_rx))
    }

    async fn consume_candles(
        &mut self,
        symbol: String,
        interval: &str,
    ) -> Result<UnboundedReceiverStream<Candle>, ClientError> {
        // Construct trades channel that ConnectionHandler will distribute trade stream data on
        let (binance_candle_tx, mut binance_candle_rx) = mpsc::unbounded_channel();

        // Construct Subscription for the ConnectionHandler to action
        let candles_subscription = BinanceSub::new(
            Binance::CANDLE_STREAM.replace("<interval>", interval),
            symbol,
        );

        // Subscribe by passing a tuple of (Subscription, trade_tx) to the ConnectionHandler
        if let Err(err) = self
            .subscription_tx
            .send((candles_subscription, binance_candle_tx))
            .await
        {
            error!("Subscription request receiver has dropped by the ConnectionHandler - closing transmitter: {:?}", err);
            return Err(ClientError::SendFailure);
        }

        // Construct channel to distribute normalised Trade data to downstream consumers
        let (candle_tx, candle_rx) = mpsc::unbounded_channel();

        // Async task to consume from binance_trades_tx and produce normalised Trades via the trades_tx
        tokio::spawn(async move {
            while let Some(binance_message) = binance_candle_rx.recv().await {
                match binance_message {
                    BinanceMessage::Kline(binance_kline) => {
                        if binance_kline.data.kline_closed == false {
                            continue;
                        }

                        if candle_tx.send(Candle::from(binance_kline)).is_err() {
                            info!(
                                "Receiver for Binance Candles has been dropped - closing stream."
                            );
                            return;
                        }
                    }
                    _ => warn!("consume_candles() received BinanceMessage that was not a Candle"),
                }
            }
        });

        // Return normalised Trade stream to consumer
        Ok(UnboundedReceiverStream::new(candle_rx))
    }
}

impl Binance {
    const BASE_URI: &'static str = "wss://stream.binance.com:9443/ws";
    const TRADE_STREAM: &'static str = "@aggTrade";
    const CANDLE_STREAM: &'static str = "@kline_<interval>";

    /// Constructs a new [Binance] [ExchangeClient] instance using the [ClientConfig] provided.
    pub async fn new(cfg: ClientConfig) -> Result<Self, ClientError> {
        // Connect to client WebSocket server
        let ws_conn = connect(&String::from(Binance::BASE_URI)).await?;

        // Construct subscription message channel to subscribe to streams via the ConnectionHandler
        let (subscription_tx, subscription_rx) = mpsc::channel(10);

        // Construct ConnectionHandler
        let connection =
            ConnectionHandler::new(cfg.rate_limit_per_minute, ws_conn, subscription_rx);

        // Manage connection via event loop
        let _ = tokio::spawn(connection.manage());

        Ok(Self { subscription_tx })
    }
}

/// [Binance] Message variants that could be received from Binance WebSocket server.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BinanceMessage {
    Subscription(BinanceSub),
    SubscriptionResponse(BinanceSubResponse),
    Trade(BinanceTrade),
    Kline(BinanceKline),
    OrderBook(BinanceOrderBook),
}

impl StreamIdentifier for BinanceMessage {
    fn get_stream_id(&self) -> Identifier {
        match self {
            BinanceMessage::Trade(trade) => Identifier::Yes(format!(
                "{}@{}",
                trade.symbol.to_lowercase(),
                trade.event_type
            )),
            BinanceMessage::Kline(kline) => Identifier::Yes(format!(
                "{}@{}_{}",
                kline.symbol.to_lowercase(),
                kline.event_type,
                kline.data.interval
            )),
            _ => Identifier::No,
        }
    }
}

/// [Binance] specific subscription message.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BinanceSub {
    method: String,
    params: Vec<String>,
    id: u64,
}

impl Subscription for BinanceSub {
    fn new(stream_name: String, ticker_pair: String) -> Self {
        Self {
            method: String::from("SUBSCRIBE"),
            params: vec![format!("{}{}", ticker_pair, stream_name)],
            id: 1,
        }
    }
}

impl StreamIdentifier for BinanceSub {
    fn get_stream_id(&self) -> Identifier {
        Identifier::Yes(self.params.get(0).unwrap().clone())
    }
}

/// [Binance] specific subscription response message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    id: u64,
}

/// [Binance] specific Trade message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E", skip_deserializing)]
    event_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "a")]
    trade_id: u64,
    #[serde(rename = "p", deserialize_with = "de_str_to_f64")]
    price: f64,
    #[serde(rename = "q", deserialize_with = "de_str_to_f64")]
    quantity: f64,
    #[serde(rename = "f", skip_deserializing)]
    buyer_order_id: u64,
    #[serde(rename = "l", skip_deserializing)]
    seller_order_id: u64,
    #[serde(rename = "T")]
    trade_time: i64,
    #[serde(rename = "m")]
    buyer_is_market_maker: bool,
    #[serde(rename = "M", skip_deserializing)]
    deprecated: bool,
}

impl From<BinanceTrade> for Trade {
    fn from(binance_trade: BinanceTrade) -> Self {
        let timestamp = DateTime::from_utc(
            NaiveDateTime::from_timestamp(binance_trade.trade_time / 1000, 0),
            Utc,
        );

        let buyer = match binance_trade.buyer_is_market_maker {
            true => BuyerType::MarketMaker,
            false => BuyerType::Taker,
        };

        Self {
            trade_id: binance_trade.trade_id.to_string(),
            timestamp,
            ticker: binance_trade.symbol,
            price: binance_trade.price,
            quantity: binance_trade.quantity,
            buyer,
        }
    }
}

/// [Binance] specific Kline message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceKline {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E", skip_deserializing)]
    event_time: i64, // todo
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "k")]
    data: BinanceKlineData,
}

/// [Binance] Kline data contained within a [BinanceKline].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceKlineData {
    #[serde(rename = "t")]
    start_time: i64,
    #[serde(rename = "T")]
    end_time: i64,
    #[serde(rename = "s", skip_deserializing)]
    symbol: String,
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "f", skip_deserializing)]
    first_trade_id: i64,
    #[serde(rename = "L", skip_deserializing)]
    last_trade_id: i64,
    #[serde(rename = "o", deserialize_with = "de_str_to_f64")]
    open: f64,
    #[serde(rename = "c", deserialize_with = "de_str_to_f64")]
    close: f64,
    #[serde(rename = "h", deserialize_with = "de_str_to_f64")]
    high: f64,
    #[serde(rename = "l", deserialize_with = "de_str_to_f64")]
    low: f64,
    #[serde(rename = "v", deserialize_with = "de_str_to_f64")]
    base_asset_volume: f64,
    #[serde(rename = "n")]
    number_trades: u64,
    #[serde(rename = "x")]
    kline_closed: bool,
    #[serde(rename = "q", deserialize_with = "de_str_to_f64", skip_deserializing)]
    quote_asset_volume: f64,
    #[serde(rename = "V", deserialize_with = "de_str_to_f64", skip_deserializing)]
    taker_buy_base_asset_volume: f64,
    #[serde(rename = "Q", deserialize_with = "de_str_to_f64", skip_deserializing)]
    taker_buy_quote_asset_volume: f64,
    #[serde(rename = "B", skip_deserializing)]
    deprecated: String,
}

impl From<BinanceKline> for Candle {
    fn from(binance_kline: BinanceKline) -> Self {
        let start_timestamp = DateTime::from_utc(
            NaiveDateTime::from_timestamp(binance_kline.data.start_time / 1000, 0),
            Utc,
        );

        let end_timestamp = DateTime::from_utc(
            NaiveDateTime::from_timestamp(binance_kline.data.end_time / 1000, 0),
            Utc,
        );

        Self {
            start_timestamp,
            end_timestamp,
            open: binance_kline.data.open,
            high: binance_kline.data.high,
            low: binance_kline.data.low,
            close: binance_kline.data.close,
            volume: binance_kline.data.base_asset_volume,
            trade_count: binance_kline.data.number_trades,
        }
    }
}

/// [Binance] specific OrderBook snapshot message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceOrderBook {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<BinanceLevel>,
    pub asks: Vec<BinanceLevel>,
}

/// [Binance] specific Level data structure used to construct a [BinanceOrderBook].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceLevel {
    #[serde(deserialize_with = "de_str_to_f64")]
    pub price: f64,
    #[serde(rename = "quantity")]
    #[serde(deserialize_with = "de_str_to_f64")]
    pub amount: f64,
}
