use crate::client::{ClientConfig, de_str_to_f64};
use crate::error::ClientError;
use crate::connection::ConnectionHandler;
use crate::{BuyerType, Candle, connect, ExchangeClient, Identifier, StreamIdentifier, Subscription, Trade};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
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
    async fn consume_trades(&mut self, symbol: String) -> Result<UnboundedReceiverStream<Trade>, ClientError> {
        // Construct trades channel that ConnectionHandler will distribute trade stream data on
        let (binance_trade_tx, mut binance_trade_rx) = mpsc::unbounded_channel();

        // Construct Subscription for the ConnectionHandler to action
        let trades_subscription = BinanceSub::new(
            String::from(Binance::TRADE_STREAM),
            symbol
        );

        // Subscribe by passing a tuple of (Subscription, trade_tx) to the ConnectionHandler
        if let Err(err) = self
            .subscription_tx
            .send((trades_subscription, binance_trade_tx))
            .await {
            error!("Subscription request receiver has dropped by the ConnectionHandler - closing transmitter: {:?}", err);
            return Err(ClientError::SendFailure)
        }

        // Construct channel to distribute normalised Trade data to downstream consumers
        let (trade_tx, trade_rx) = mpsc::unbounded_channel();

        // Async task to consume from binance_trades_tx and produce normalised Trades via the trades_tx
        tokio::spawn(async move {
            while let Some(binance_message) = binance_trade_rx.recv().await {
                match binance_message {
                    BinanceMessage::Trade(binance_trade) => {
                        if trade_tx.send(Trade::from(binance_trade)).is_err() {
                            info!("Receiver for Binance Trades has been dropped - closing stream.");
                            return;
                        }
                    },
                    _ => warn!("consume_trades() received BinanceMessage that was not a Trade")
                }
            }
        });

        // Return normalised Trade stream to consumer
        Ok(UnboundedReceiverStream::new(trade_rx))
    }

    async fn consume_candles(&mut self, symbol: String, interval: &str) -> Result<UnboundedReceiverStream<Candle>, ClientError> {
        // Construct trades channel that ConnectionHandler will distribute trade stream data on
        let (binance_candle_tx, mut binance_candle_rx) = mpsc::unbounded_channel();

        // Construct Subscription for the ConnectionHandler to action
        let candles_subscription = BinanceSub::new(
            Binance::CANDLE_STREAM.replace("<interval>", interval),
            symbol
        );

        // Subscribe by passing a tuple of (Subscription, trade_tx) to the ConnectionHandler
        if let Err(err) = self
            .subscription_tx
            .send((candles_subscription, binance_candle_tx))
            .await {
            error!("Subscription request receiver has dropped by the ConnectionHandler - closing transmitter: {:?}", err);
            return Err(ClientError::SendFailure)
        }

        // Construct channel to distribute normalised Trade data to downstream consumers
        let (candle_tx, candle_rx) = mpsc::unbounded_channel();

        // Async task to consume from binance_trades_tx and produce normalised Trades via the trades_tx
        tokio::spawn(async move {
            while let Some(binance_message) = binance_candle_rx.recv().await {
                match binance_message {
                    BinanceMessage::Candle(binance_candle) => {
                        if candle_tx.send(Candle::from(binance_candle)).is_err() {
                            info!("Receiver for Binance Candles has been dropped - closing stream.");
                            return;
                        }
                    },
                    _ => warn!("consume_candles() received BinanceMessage that was not a Candle")
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
        let connection = ConnectionHandler::new(
            cfg.rate_limit_per_second,
            ws_conn, subscription_rx
        );

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
    Candle(BinanceCandle),
    OrderBook(BinanceOrderBook)
}

impl StreamIdentifier for BinanceMessage {
    fn get_stream_id(&self) -> Identifier {
        match self {
            BinanceMessage::Trade(trade) => {
                Identifier::Yes(format!("{}@{}", trade.s.to_lowercase(), trade.e))
            },
            _ => Identifier::No
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
    /// Event Type
    e: String,
    /// Event Time
    #[serde(rename = "E")]
    et: u64,
    /// Symbol
    s: String,
    /// Trade Id
    a: u64,
    /// Price
    #[serde(deserialize_with = "de_str_to_f64")]
    p: f64,
    /// Quantity
    #[serde(deserialize_with = "de_str_to_f64")]
    q: f64,
    /// Buyer Order Id
    f: u64,
    /// Seller Order Id
    l: u64,
    /// Trade Time
    #[serde(rename = "T")]
    t: u64,
    /// Buyer Is Market Maker
    m: bool,
    /// Deprecated
    #[serde(rename = "M")]
    deprecated: bool
}

impl From<BinanceTrade> for Trade {
    fn from(binance_trade: BinanceTrade) -> Self {
        let buyer = match binance_trade.m {
            true => BuyerType::Maker,
            false => BuyerType::Taker,
        };

        Self {
            trade_id: binance_trade.a.to_string(),
            timestamp: binance_trade.t.to_string(),
            ticker: binance_trade.s,
            price: binance_trade.p,
            quantity: binance_trade.q,
            buyer,
        }
    }
}

/// [Binance] specific Candle message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceCandle {
    /// Event Type
    e: String,
    /// Event Time
    #[serde(rename = "E")]
    et: i64,
    /// Symbol
    s: String,
    /// Data
    k: BinanceCandleData,
}

/// [Binance] Candle data contained within a [BinanceCandle].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceCandleData {
    /// Kline Start Time
    t: u64,
    /// Kline Close Time
    #[serde(rename = "T")]
    ct: i64,
    /// Symbol
    s: String,
    /// Interval
    i: String,
    /// First Trade Id
    f: u64,
    /// Last Trade Id
    #[serde(rename = "L")]
    lt: u64,
    /// Open Price
    o: f64,
    /// Close Price
    c: f64,
    /// High Price
    h: f64,
    /// Low Price
    l: f64,
    /// Base Asset Volume
    v: u64,
    /// Number Of Trades
    n: u64,
    /// Is This Kline/Candlestick Closed
    x: bool,
    /// Quote Asset Volume
    q: f64,
    /// Taker Buy Base Asset Volume
    #[serde(rename = "V")]
    tv: u64,
    /// Taker Buy Quote Asset Volume
    qv: f64,
    /// Deprecated
    #[serde(rename = "B")]
    b: u64,
}

impl From<BinanceCandle> for Candle {
    fn from(binance_candle: BinanceCandle) -> Self {
        let timestamp = DateTime::from_utc(
            NaiveDateTime::from_timestamp(binance_candle.k.ct, 0), Utc
        );

        Self {
            timestamp,
            open: binance_candle.k.o,
            high: binance_candle.k.h,
            low: binance_candle.k.l,
            close: binance_candle.k.c,
            volume: binance_candle.k.v as f64,
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