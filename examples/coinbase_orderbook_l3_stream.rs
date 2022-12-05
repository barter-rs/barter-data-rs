// standard
use std::{
    sync::Arc,
    collections::HashMap,
    env,
    path::PathBuf,
};
// external
use chrono::Utc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
};
use tracing_appender::non_blocking::WorkerGuard;
use anyhow;
use futures::{stream::FuturesUnordered, StreamExt};
use tokio::{
    fs::File,
    signal,
    sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, RwLock},
    time::Duration,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
};
// adjacent
use barter_integration::model::{InstrumentKind, Market, MarketId, Side};
use coinbase_pro_api::*;
// internal
use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind},
        orderbook::OrderBookL3,
        DataKind,
    },
    shutdown::{shutdown_channel, ShutdownListener, ShutdownNotifier},
    exchange::coinbase::model::CoinbaseOrderBookL3Snapshot,
    model::{
        OrderBookL3Snapshot,
        orderbook::OrderBookEvent,
    }
};

// Parameters
static LOG_FOLDER: &str = "logs/";
static SNAPSHOT_REQUEST_DELAY: Duration = Duration::from_millis(1000); // seconds
static SAVE_STREAM: bool = true;
static LOAD_STREAM: bool = false;
static LOAD_STREAM_PATH: &str = "data/2022-10-24UTC/stream_dump_coinbase_(eth_usd, spot)_20221024_210235.jsonl";

/// Send kill signal when hitting ctrl+c for graceful shutdown
async fn ctrl_c_watch(mut shutdown: ShutdownNotifier) {
    signal::ctrl_c().await.unwrap();
    shutdown.send();
}

/// reference: https://github.com/tokio-rs/tracing/issues/971#issuecomment-690045204
async fn init_tracing() -> WorkerGuard {
    tokio::fs::create_dir_all(LOG_FOLDER).await.expect("Failed to create log directory");
    let file_appender
        = tracing_appender::rolling::daily(LOG_FOLDER, "orderbook_l3_stream_log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish()
            .with(fmt::Layer::default().with_writer(file_writer))
    ).expect("Unable to set global tracing subscriber");
    debug!("Tracing initialized.");
    guard
}

#[tokio::main]
async fn main() {

    if SAVE_STREAM && LOAD_STREAM {
        panic!("Cannot save and load stream simultaneously! Adjust SAVE_STREAM, LOAD_STREAM params.");
    }

    // logging to file and stdout
    let _guard = init_tracing().await;

    let (event_tx, event_rx)
        = unbounded_channel::<MarketEvent>();

    let subscriptions: Vec<Subscription> = vec![
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
        // (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade).into(),
        // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
    ];

    let (stop_tx, stop_rx) = shutdown_channel();

    let stream_thread = if LOAD_STREAM {
        tokio::spawn(run_local_stream(stop_rx, event_tx))
    } else {
        tokio::spawn(run_streams(subscriptions.clone(), stop_rx, event_tx))
    };

    let ob_thread = tokio::spawn(
        orderbook_handler(subscriptions, event_rx));
    let stop_thread = tokio::spawn(ctrl_c_watch(stop_tx));

    stop_thread.await.unwrap();
    stream_thread.await.unwrap();
    ob_thread.await.unwrap();
}

async fn orderbook_handler(
    subscriptions: Vec<Subscription>,
    mut event_rx: UnboundedReceiver<MarketEvent>,
) {
    let mut tasks = FuturesUnordered::new();

    // initialize coinbase public REST api client
    let cbp_pub_client = Arc::new(CoinbasePublicClient::builder().build());

    let mut orderbook_map: HashMap<Market, OrderBookL3> = HashMap::new();

    // initialize orderbook for each subscription and insert into orderbook map
    subscriptions.iter()
        .filter(|subscription| subscription.kind == SubKind::OrderBookL3Delta)
        .for_each( |subscription| {
            let market = Market::from(
                (subscription.exchange.clone(), subscription.instrument.clone())
            );
            let orderbook = OrderBookL3::builder()
                .market(market.clone())
                .stats(false)
                // .add_panic_button()
                // .last_n_events(5)
                .build()
                .unwrap();
            orderbook_map.insert( market,orderbook);
        }
    );

    // construct handler map
    let mut handler_map: HashMap<Market, OrderbookHandler> = HashMap::new();
    orderbook_map.into_iter().for_each(|(market, orderbook)| {
        let (ob_event_tx, ob_event_rx)
            = unbounded_channel::<MarketEvent>();
        handler_map.insert(market.clone(), OrderbookHandler::build(orderbook, ob_event_tx));
        let handler = handler_map.get_mut(&market).unwrap();
        let orderbook_mut = handler.orderbook.clone();
        tasks.push(tokio::spawn(
            process_orderbook(cbp_pub_client.clone(), orderbook_mut, ob_event_rx)
        ))
    });

    // feed market events into their respective orderbook channels
    while let Some(market_event) = event_rx.recv().await {
        let market = Market::from(
            (market_event.exchange.clone(), market_event.instrument.clone())
        );
        if let Some(handler) = handler_map.get_mut(&market) {
            handler.sender.as_mut().unwrap().send(market_event).unwrap();
        }
    }

    // close all senders
    debug!("Orderbook handler broke market event loop");
    for (_market, mut handler) in handler_map {
        handler.sender.take();
    }

    // wait for child processes to conclude
    debug!("Awaiting orderbook processors to conclude...");
    while let Some(_task) = tasks.next().await {}
    debug!("Orderbook processing finished.")
}

async fn process_orderbook(
    client: Arc<CoinbasePublicClient>,
    orderbook: Arc<RwLock<OrderBookL3>>,
    mut ob_event_rx: UnboundedReceiver<MarketEvent>,
) {

    // get orderbook's market
    let market: Market = {
        let reader = orderbook.read().await;
        reader.market.clone()
    };
    let market_id: MarketId = MarketId::from(&market);

    // initialize stream dumper, if enabled
    let mut stream_dumper: Option<StreamDump> = match SAVE_STREAM {
        true => {
            debug!("initiating stream dump");
            StreamDump::init(&market).await.ok()
        }
        false => { None }
    };

    // attempt to fetch snapshot and load
    match fetch_snapshot_via_api(client, orderbook.clone(), &stream_dumper).await {
        Ok(snapshot) => {
            // load snapshot into orderbook
            debug!("Awaiting orderbook write lock...");
            let mut writer  = orderbook.write().await;
            let snapshot_sequence = snapshot.sequence.clone();
            let _ = writer.process( MarketEvent {
                exchange_time: Utc::now(), // fake exchange time
                received_time: Utc::now(),
                exchange: market.exchange.clone(),
                instrument: market.instrument.clone(),
                kind: DataKind::OBEvent(OrderBookEvent::Snapshot(snapshot, snapshot_sequence))
            });
            debug!("Loaded snapshot for {}. Starting sequence = {}. Commencing main event processing",
               market_id , snapshot_sequence);
        },
        Err(error) => {
            error!("Error fetching snapshot: {}", error);
        }
    }

    // main orderbook processing loop
    while let Some(event) = ob_event_rx.recv().await {
        let mut writer = orderbook.write().await;

        if stream_dumper.is_some() {
            stream_dumper.as_mut().unwrap().dump(&event).await;
        }

        let _ = writer.process(event);
    }

    // finish
    debug!("Orderbook processor has finished for {:?}", market);
    let reader = orderbook.read().await;
    reader.print_info(false);
}

async fn fetch_snapshot_via_api(
    client: Arc<CoinbasePublicClient>,
    orderbook: Arc<RwLock<OrderBookL3>>,
    stream_dumper: &Option<StreamDump>
) -> Result<OrderBookL3Snapshot, anyhow::Error> {

    // derive product_id str from orderbook's market
    let product_id_str: String = {
        let reader = orderbook.read().await;
        format!("{}-{}", reader.market.instrument.base, reader.market.instrument.quote)
    };

    // Coinbase's snapshot timing requires a non-deterministic delay for sequence syncing
    info!("Requesting full orderbook snapshot of {} from Coinbase in {} seconds...",
        product_id_str, SNAPSHOT_REQUEST_DELAY.as_secs_f64());
    tokio::time::sleep(SNAPSHOT_REQUEST_DELAY).await;

    // get snapshot
    let snapshot_str = client
        .get_product_orderbook(&product_id_str, OBLevel::Level3).await?;

    // deserialize into snapshot struct
    let snapshot: OrderBookL3Snapshot
        = serde_json::from_str::<CoinbaseOrderBookL3Snapshot>(&snapshot_str)?.into();
    info!("Retrieved full orderbook snapshot of {} from Coinbase.", product_id_str);

    // save snapshot
    if stream_dumper.is_some() {
        match stream_dumper.as_ref().unwrap().save_orderbook_snapshot(snapshot_str).await {
            Err(error) => warn!("Snapshot save failed: {error}"),
            _ => {},
        }
    }

    Ok(snapshot)
}

async fn run_streams(
    subscriptions: Vec<Subscription>,
    stop_rx: ShutdownListener,
    event_tx: UnboundedSender<MarketEvent>,
) {
    let streams = Streams::builder()
        .subscribe(subscriptions)
        .graceful_kill(stop_rx)
        .init().await.unwrap();

    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    while let Some((_exchange, market_event)) = joined_stream.next().await {
        pretty_print_trade(&market_event);

        let _ = event_tx.send(market_event);
    }

    joined_stream.clear();

    debug!("Stream thread ending.");
}


async fn run_local_stream(
    _stop_rx: ShutdownListener,
    event_tx: UnboundedSender<MarketEvent>,
) {
    let mut reader = StreamRead::init(LOAD_STREAM_PATH).await.unwrap();

    while let Some(market_event) = reader.read().await {
        let _ = event_tx.send(market_event);
    }

    debug!("Local stream thread ending.");

}

fn pretty_print_trade(event: &MarketEvent) {
    match &event.kind {
        DataKind::Trade(trade) => {
            let (side, price, volume) = match trade.side {
                Side::Buy => {(
                    "Buy",
                    format!("{:.2}", trade.price),
                    format!("${:.2}", (trade.quantity * trade.price)),
                )},
                Side::Sell => {(
                    "Sell",
                    format!("{:.2}", trade.price),
                    format!("${:.2}", (trade.quantity * trade.price)),
                )},
            };
            let left_align = format!(
                "{} --- {} {} {}-{} at ${}",
                event.exchange_time,
                side,
                trade.quantity,
                event.instrument.base,
                event.instrument.quote,
                price,
            );
            println!("{:<80} {:>25}",left_align, volume);
        },
        _ => {}
    }
}

struct OrderbookHandler {
    orderbook: Arc<RwLock<OrderBookL3>>,
    sender: Option<UnboundedSender<MarketEvent>>,
}

impl OrderbookHandler {
    fn build(
        orderbook: OrderBookL3,
        sender: UnboundedSender<MarketEvent>,
    ) -> Self {
        Self {
            orderbook: Arc::new(RwLock::new(orderbook)),
            sender: Some(sender),
        }
    }
}

/// Struct that reads market events from a jsonl file
/// Todo: add support for multiple stream dumps
pub struct StreamRead {
    // pub stream_path: PathBuf,
    // pub snapshot_path: Option<PathBuf>,
    pub reader: BufReader<File>,
}

impl StreamRead {
    pub async fn init(stream_path: &str) -> std::io::Result<Self> {
        let reader = BufReader::new(File::open(stream_path).await?);
        Ok(Self { reader })
    }

    pub async fn read(&mut self) -> Option<MarketEvent> {
        let mut line: String = String::new();
        let len = self.reader.read_line(&mut line).await;
        match len {
            Ok(n) if n > 0 => {
                let market_event = serde_json::from_str::<MarketEvent>(&*line);
                market_event.ok()
            },
            _ => None
        }
    }
}

/// Struct that writes market events into a jsonl file. Also saves snapshot in a json.
pub struct StreamDump {
    pub market: Market,
    pub path: PathBuf,
    pub stream_path: PathBuf,
    pub writer: BufWriter<File>,
}

impl StreamDump {
    pub async fn init(market: &Market) -> std::io::Result<Self> {
        // open new file for dumping
        let mut path = env::current_dir()?;
        path.push(format!("data\\{}\\", Utc::today().to_string()));
        tokio::fs::create_dir_all(path.clone()).await?;
        let filename = Self::make_stream_dump_filename(&market);
        let mut filepath = path.clone();
        filepath.push(filename);
        info!("Stream dumper writing to {:?}", filepath);
        let file = File::create(&filepath).await?;
        let writer = BufWriter::new(file);
        Ok(Self { market: market.clone(), path, stream_path: filepath, writer })
    }

    pub async fn dump(&mut self, event: &MarketEvent) {
        let mut event_str = serde_json::to_string(event).unwrap();
        event_str += "\n";
        // let event_str = format!("{}\n", event_str);
        // debug!("dumped {}", event_str);
        match self.writer.write_all(event_str.as_bytes()).await {
            Err(error) => {
                warn!("Stream dumper encountered error: {}", error);
            },
            _ => {},
        };
    }

    pub async fn save_orderbook_snapshot(&self, snapshot:String) -> std::io::Result<()> {
        let filename = Self::make_snapshot_filename(&self.market);
        let mut filepath = self.path.clone();
        filepath.push(filename);
        info!("Saving snapshot into {:?}", filepath);
        let mut f = tokio::fs::File::create(filepath).await?;
        f.write_all(snapshot.as_bytes()).await?;
        Ok(())
    }

    fn make_snapshot_filename(market: &Market) -> String {
        let market_id = MarketId::from(market);
        format!(
            "snapshot_{}_{}.json",
            market_id,
            Utc::now().format("%Y%m%d_%H%M%S").to_string()
        )
    }

    fn make_stream_dump_filename(market: &Market) -> String {
        let market_id = MarketId::from(market);
        format!(
            "market_events_{}_{}.jsonl",
            market_id,
            Utc::now().format("%Y%m%d_%H%M%S").to_string()
        )
    }
}

impl Drop for StreamDump {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}