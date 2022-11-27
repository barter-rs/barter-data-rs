use std::{time::Duration, collections::BTreeMap, sync::{Arc, RwLock}};

use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{
        MarketEvent,
        subscription::{SubKind}, OrderBookBTreeMap, OrderBookL2Update, DataKind
    }, exchange::{kucoin::model::{KucoinLevel}, de_u64_epoch_ms_as_datetime_utc},
};
use barter_integration::model::InstrumentKind;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use ordered_float::OrderedFloat;
use reqwest::{header::{HeaderMap, HeaderValue, HeaderName}, Response, StatusCode};
use anyhow::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tokio::{task::JoinHandle, sync::mpsc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::encode;

// -------------------------------------

type HmacSha256 = Hmac<Sha256>;

/// Build a request given the parameters as a map.
pub fn build_request(parameters: &BTreeMap<String, String>) -> String {
    let mut request = String::new();
    for (key, value) in parameters {
        let param = format!("{}={}&", key, value);
        request.push_str(param.as_ref());
    }
    request.pop(); // remove last &

    request
}

#[derive(Clone, Debug)]
/// A HTTPS Client connection.
pub struct KucoinClient {
    /// THe API keys as provided by the exchange.
    api_key: String,
    /// The API private key as provided by the exchange.
    secret_key: String,
    /// The API passphrase,
    api_passphrase: String,
    /// The inner reqwest client.
    inner: reqwest::Client,
    /// The host endpoint.
    host: String,
}

impl KucoinClient {
    /// Returns a client based on the specified host and credentials
    /// Credentials do not need to be specified when using public endpoints
    /// Host is mandatory
    pub fn new(api_key: Option<String>, secret_key: Option<String>, passphrase: Option<String>, host: String) -> Self {
        let builder: reqwest::ClientBuilder = reqwest::ClientBuilder::new();
        let builder = builder.timeout(Duration::from_secs(2));
        KucoinClient {
            api_key: api_key.unwrap_or_else(|| "".into()),
            secret_key: secret_key.unwrap_or_else(|| "".into()),
            api_passphrase: passphrase.unwrap_or_else(|| "".into()),
            inner: builder.build().unwrap(),
            host,
        }
    }

    /// Get the timestamp
    fn get_timestamp() -> Result<u64, anyhow::Error> {
        Ok(Utc::now().timestamp_millis() as u64)
    }

    /// Send a signed GET request.
    ///
    /// # Arguments
    /// * 'endpoint' - The API endpoint.
    /// * 'request' - The request parameter string.
    ///
    ///  ## Return
    /// * 'Ok(String)' - The response string from the endpoint.
    /// * 'Err(anyhow::Error)' - Generic response string.
    pub async fn get_signed(&self, endpoint: &str, request: &str) -> Result<String> {
        // Build the url
        let mut url: String = format!("{}{}", self.host, endpoint);
        if !request.is_empty() {
            url.push_str(format!("?{}", request).as_str());
        }
        // Build the headers
        let headers = self.build_headers(&format!("{}?{}", endpoint, request));
        // Send the request
        let response = self
            .inner
            .clone()
            .get(url.as_str())
            .headers(headers?)
            .send()
            .await?;

        self.handler(response).await
    }

    /// Send a GET request.
    ///
    /// # Arguments
    /// * 'endpoint' - The API endpoint.
    /// * 'request' - The request parameter string.
    ///
    /// ## Return
    /// * 'Ok(String)' - The response string from the endpoint.
    /// * 'Err(anyhow::Error)' - Generic response string.
    pub async fn get(&self, endpoint: &str, request: &str) -> Result<String> {
        let mut url: String = format!("{}{}", self.host, endpoint);
        if !request.is_empty() {
            url.push_str(format!("?{}", request).as_str());
        }

        let response = reqwest::get(url.as_str()).await?;

        self.handler(response).await
    }

    /// Build the header for the request
    fn build_headers(&self, endpoint: &str) -> Result<HeaderMap> {
        let mut custom_headers = HeaderMap::new();

        let timestamp = &Self::get_timestamp()?.to_string();

        let str_to_sign = format!("{timestamp}GET{endpoint}");

        let mut hmac_sign = HmacSha256::new_varkey(self.secret_key.as_bytes()).expect("HMAC can take key of any size");
        hmac_sign.input(str_to_sign.as_bytes());
        let sign_result = hmac_sign.result();
        let sign_bytes = sign_result.code();
        let sign_digest = encode(&sign_bytes);
        let mut hmac_passphrase = HmacSha256::new_varkey(self.secret_key.as_bytes()).expect("HMAC can take key of any size");
        hmac_passphrase.input(self.api_passphrase.as_bytes());
        let passphrase_result = hmac_passphrase.result();
        let passphrase_bytes = passphrase_result.code();
        let passphrase_digest = encode(&passphrase_bytes);

        custom_headers.insert(
            HeaderName::from_static("kc-api-key"),
            HeaderValue::from_str(&self.api_key).unwrap(),
        );
        custom_headers.insert(
            HeaderName::from_static("kc-api-timestamp"), 
            HeaderValue::from_str(timestamp)?
        );
        custom_headers.insert(
            HeaderName::from_static("kc-api-passphrase"), 
            HeaderValue::from_str(&passphrase_digest).unwrap()
        );
        custom_headers.insert(
            HeaderName::from_static("kc-api-key-version"), 
            HeaderValue::from_str("2").unwrap()
        );
        custom_headers.insert(
            HeaderName::from_static("kc-api-sign"), 
            HeaderValue::from_str(&sign_digest).unwrap(),
        );

        Ok(custom_headers)
    }

    /// Handle the API response based on the status code.
    async fn handler(&self, response: Response) -> Result<String> {
        match response.status() {
            StatusCode::OK => {
                let body = response.bytes().await?;
                let result = std::str::from_utf8(&body)?;
                //response.json::<T>().await.or_else(|_| Err(anyhow!("ERROR PARSING JSON")))
                Ok(result.to_string())
            }
            StatusCode::INTERNAL_SERVER_ERROR => Err(anyhow!("INTERNAL_SERVER_ERROR")),
            StatusCode::SERVICE_UNAVAILABLE => Err(anyhow!("SERVICE_UNAVAILABLE")),
            StatusCode::UNAUTHORIZED => Err(anyhow!("UNAUTHORIZED")),
            StatusCode::BAD_REQUEST => Err(anyhow!("BAD_REQUEST")),
            s => Err(anyhow!("Received response: {:?}", s)),
        }
    }
}
// -------------------------------------
/// ['Kucoin'](super::Kucoin) level 2 orderbook snapshot. Can be depth5 or depth50.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct KucL2Snapshot {
    #[serde(deserialize_with = "de_u64_epoch_ms_as_datetime_utc")]
    pub time: DateTime<Utc>,
    #[serde(deserialize_with = "barter_data::exchange::de_str")]
    pub sequence: u64,
    pub bids: Vec<KucoinLevel>,
    pub asks: Vec<KucoinLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
struct OrderBookREST {
    data: KucL2Snapshot
}

impl Into<OrderBookBTreeMap> for OrderBookREST {
    fn into(self) -> OrderBookBTreeMap {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();

        

        for bid in self.data.bids {
            bids.insert(OrderedFloat(bid.price), bid.quantity);
        }

        for ask in self.data.asks {
            asks.insert(OrderedFloat(ask.price), ask.quantity);
        }

        OrderBookBTreeMap { last_update_id: self.data.sequence, bids, asks }
    }
}

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create data stream
    let streams = Streams::builder()
        .subscribe([
            (ExchangeId::Kucoin, "btc", "usdt", InstrumentKind::Spot, SubKind::OrderBookL2Update),
        ])
        .init()
        .await
        .unwrap();

    // Create data channel
    let (data_tx_orig, mut data_rx) = mpsc::unbounded_channel::<OrderBookL2Update>();

    // Spawn a task for receiving updates
    let data_tx = data_tx_orig.clone();
    let receive_task: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        // Join all exchange streams into a StreamMap
        // Note: Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
        let mut joined_stream = streams.join_map::<MarketEvent>().await;

        while let Some((_, event)) = joined_stream.next().await {
            match event.kind {
                DataKind::OrderBookL2Update(update) => {
                    data_tx.send(update)?;
                },
                _ => panic!("Should not be receiving other market data"),
            }
        }
        Ok(())
    });

    // Sleep so that we can accrue enough updates
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get initial orderbook snapshot
    let client = KucoinClient::new(
        Some("62ba79afff18da0001ded207".into()), 
        Some("781b987f-2edb-4aec-91b1-9d3ea22d6cd2".into()), 
        Some("48315731".into()),
        "https://api.kucoin.com".to_string()
    );
    let mut map = BTreeMap::new();
    map.insert("symbol".to_string(), "BTC-USDT".to_string());
    let request = build_request(&map);
    // let data = client.get("/api/v1/market/orderbook/level2_100", &request).await.unwrap();
    let data = client.get_signed("/api/v3/market/orderbook/level2", &request).await.unwrap();
    let data: OrderBookREST = from_str(&data).unwrap();
    let orderbook: OrderBookBTreeMap = data.into();
    let orderbook_orig = Arc::new(RwLock::new(orderbook));

    // Spawn a task for applying updates to orderbook
    let orderbook = orderbook_orig.clone();
    let update_task: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        while let Some(update) = data_rx.recv().await {
            let mut lock = orderbook.write().unwrap();
            if lock.last_update_id < update.sequence_num - 1 {
                println!("Sequence number was skipped --> book_number: {}, update_num: {}", lock.last_update_id, update.sequence_num);
            }
            if lock.last_update_id < update.sequence_num {
                lock.apply_update(&update);
            } else {
                println!("book last_seq: {}, update_id: {}", lock.last_update_id, update.sequence_num);
            }
        }
        Ok(())
    });

    // Spawn a task for printing BBO
    let orderbook = orderbook_orig.clone();
    let bbo_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let lock = orderbook.read().unwrap();
            let bid = lock.get_best_bid().unwrap();
            let ask = lock.get_best_ask().unwrap();
            let mid = round::round((ask+bid)/2.0, 2);
            let spread = round::round(ask - bid, 2);
            println!("bid: {bid}, ask: {ask}, spread: {spread}, mid: {mid}");
        }
    });

    // Join all the tasks
    let (update_task, receive_task) = tokio::join!(update_task, receive_task);
    update_task??;
    receive_task??;

    Ok(())
}