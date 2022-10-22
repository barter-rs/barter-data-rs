#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::collections::{HashMap};
use std::slice::Iter;
use std::sync::Arc;
use std::{env, fs};
use std::io::{BufWriter};
use std::path::PathBuf;
use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind},
        orderbook::OrderBookL3,
        DataKind,
    },
    shutdown::{shutdown_channel, ShutdownListener, ShutdownNotifier}
};
use barter_integration::model::{InstrumentKind, Market, Side};
use chrono::Utc;
use futures::StreamExt;
use tokio::{
    signal,
    sync::{watch, mpsc::unbounded_channel, RwLock},
    time::{sleep, Duration},
};
use tracing_subscriber;
use tracing::{debug, info, Level, warn};
use coinbase_pro_api::*;
use futures::stream::FuturesUnordered;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use barter_data::exchange::coinbase::model::CoinbaseOrderBookL3Snapshot;
use barter_data::model::orderbook::OrderBookEvent;

use barter_data::model::OrderBookL3Snapshot;

static SNAPSHOT_REQUEST_DELAY: Duration = Duration::from_millis(1000); // seconds
static SAVE_STREAM: bool = true;

/// Send kill signal when hitting ctrl+c for graceful shutdown
async fn ctrl_c_watch(mut shutdown: ShutdownNotifier) {
    signal::ctrl_c().await.unwrap();
    shutdown.send();
}

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let (event_tx, event_rx)
        = unbounded_channel::<MarketEvent>();

    let subscriptions: Vec<Subscription> = vec![
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
        // (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade).into(),
        // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
    ];

    let (stop_tx, stop_rx) = shutdown_channel();

    let stream_thread = tokio::spawn(
        run_streams(subscriptions.clone(), stop_rx, event_tx));
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
    while let Some(task) = tasks.next().await {}
    debug!("Orderbook processing finished.")
}

async fn process_orderbook(
    client: Arc<CoinbasePublicClient>,
    orderbook: Arc<RwLock<OrderBookL3>>,
    mut ob_event_rx: UnboundedReceiver<MarketEvent>,
) {

    let product_id_str: String;
    let market: Market;
    {
        let reader = orderbook.read().await;
        product_id_str = format!("{}-{}", reader.market.instrument.base, reader.market.instrument.quote);
        market = reader.market.clone();
    }

    let mut stream_dumper: Option<StreamDump> = match SAVE_STREAM {
        true => {
            debug!("initiating stream doooomper");
            StreamDump::init(&market).await.ok()
        }
        false => { None }
    };


    // ------------------------------ Load Snapshot ----------------------------------
    //
    // info!("Requesting full orderbook snapshot of {} from Coinbase in {} seconds...",
    //     product_id_str, SNAPSHOT_REQUEST_DELAY.as_secs_f64());
    // tokio::time::sleep(SNAPSHOT_REQUEST_DELAY).await;
    //
    // let snapshot_str = client
    //     .get_product_orderbook(&product_id_str, OBLevel::Level3)
    //     .await.unwrap();
    //
    // let snapshot: OrderBookL3Snapshot = serde_json::from_str::<CoinbaseOrderBookL3Snapshot>(&snapshot_str)
    //     .unwrap().into();
    // info!("Retrieved full orderbook snapshot of {} from Coinbase.", product_id_str);
    //
    // // optionally, save the snapshot into a text file
    // if SAVE_SNAPSHOT { let _ = save_orderbook_snapshot(&market, snapshot_str); }
    //
    // // load snapshot into orderbook
    // {
    //     debug!("Awaiting orderbook write lock...");
    //     let mut writer  = orderbook.write().await;
    //     let snapshot_sequence = snapshot.sequence.clone();
    //     let _ = writer.process( MarketEvent {
    //         exchange_time: Utc::now(), // fake exchange time
    //         received_time: Utc::now(),
    //         exchange: market.exchange.clone(),
    //         instrument: market.instrument.clone(),
    //         kind: DataKind::OBEvent(OrderBookEvent::Snapshot(snapshot, snapshot_sequence))
    //     });
    //     debug!("Loaded snapshot for {}. Starting sequence = {}. Commencing main event processing",
    //         product_id_str, snapshot_sequence);
    // }

    // -------------------------------------------------------------------------

    // -------------------- Main Market Event Loop -----------------------------

    while let Some(event) = ob_event_rx.recv().await {
        let mut writer = orderbook.write().await;


        if stream_dumper.is_some() {
            stream_dumper.as_mut().unwrap().dump(&event).await;
        }

        let _ = writer.process(event);
    }
    debug!("Orderbook processor has finished for {:?}", market);
    let reader = orderbook.read().await;
    reader.print_info(false);
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

    while let Some((exchange, market_event)) = joined_stream.next().await {
        // pretty_print_trade(&market_event);

        let _ = event_tx.send(market_event);
    }

    joined_stream.clear();

    debug!("Stream thread ending.");
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
    join_handle: Option<JoinHandle<()>>,
}

impl OrderbookHandler {
    fn build(
        orderbook: OrderBookL3,
        sender: UnboundedSender<MarketEvent>,
    ) -> Self {
        Self {
            orderbook: Arc::new(RwLock::new(orderbook)),
            sender: Some(sender),
            join_handle: None
        }
    }
}

/// Struct with dump implementation that serializes market events
pub struct StreamDump {
    pub market: Market,
    pub path: PathBuf,
    pub file: File,
    pub buffer: BufWriter<MarketEvent>,
}

impl StreamDump {
    pub async fn init(market: &Market) -> std::io::Result<Self> {
        // open new file for dumping
        let mut path = env::current_dir()?;
        path.push(format!("data\\{}\\", Utc::today().to_string()));
        fs::create_dir_all(path.clone())?;
        let filename = Self::make_stream_dump_filename(&path, &market);
        path.push(filename);
        info!("Stream dumper writing to {:?}", path);
        let mut file = File::create(&path).await?;
        Ok(Self { market: market.clone(), path, file })
    }

    pub async fn dump(&mut self, event: &MarketEvent) {
        let event_str = format!("{:?},\n", event);
        debug!("dumped {}", event_str);
        match self.file.write_all(event_str.as_bytes()).await {
            Err(error) => {
                warn!("Stream dumper encountered error: {}", error);
            },
            _ => {},
        };
    }

    pub async fn save_orderbook_snapshot(&self, snapshot:String) -> std::io::Result<()> {
        let filepath = Self::make_snapshot_filename(&self.path, &self.market);
        info!("Saving snapshot into {}", filepath.to_string());
        let mut f = fs::File::create(filepath)?;
        f.write_all(snapshot.as_bytes())?;
        Ok(())
    }

    fn make_snapshot_filename(path: &PathBuf, market: &Market) -> String {
        format!(
            "snapshot_{}_{}_{}.json",
            market.exchange,
            market.instrument,
            Utc::now().format("%Y%m%d_%H%M%S").to_string()
        )
    }

    fn make_stream_dump_filename(path: &PathBuf, market: &Market) -> String {
        format!(
            "stream_dump_{}_{}_{}.json",
            market.exchange,
            market.instrument,
            Utc::now().format("%Y%m%d_%H%M%S").to_string()
        )
    }
}