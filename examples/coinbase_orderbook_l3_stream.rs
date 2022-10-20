use std::collections::{HashMap};
use std::slice::Iter;
use std::sync::Arc;
use std::{env, fs};
use std::io::Write;
use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind},
        orderbook::OrderBookL3,
        DataKind,
    },
};

use barter_integration::model::{InstrumentKind, Market, Side};
use chrono::Utc;
use futures::StreamExt;
use tokio::{
    signal,
    sync::{oneshot, mpsc::unbounded_channel, RwLock},
    time::{sleep, Duration},
};
use tracing_subscriber;
use tracing::info;
use coinbase_pro_api::*;
use futures::stream::FuturesUnordered;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use barter_data::exchange::coinbase::model::CoinbaseOrderBookL3Snapshot;
use barter_data::model::DataKind::OBEvent;
use barter_data::model::orderbook::OrderBookEvent;

use barter_data::model::OrderBookL3Snapshot;

static SNAPSHOT_REQUEST_DELAY: Duration = Duration::from_millis(1000); // seconds
static SAVE_SNAPSHOT: bool = true;

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt().init();

    let (event_tx, event_rx)
        = unbounded_channel::<MarketEvent>();

    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    let subscriptions: Vec<Subscription> = vec![
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
        // (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade).into(),
        // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
    ];

    let _stream_thread = tokio::spawn(run_streams(subscriptions.clone(), stop_rx, event_tx));
    let _ob_thread = tokio::spawn(orderbook_handler(subscriptions, event_rx));
    let stop_thread = tokio::spawn(ctrl_c_watch(stop_tx));

    stop_thread.await.unwrap();

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
                .last_n_events(5)
                .build()
                .unwrap();
            orderbook_map.insert( market,orderbook);
        }
    );

    // construct handler map
    let mut handler_map: HashMap<Market, OrderbookHandler> = HashMap::new();
    orderbook_map.into_iter().for_each(|(market, orderbook)| {
        let (mut ob_event_tx, mut ob_event_rx)
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
    info!("Orderbook handler broke market event loop");
    for (_market, mut handler) in handler_map {
        handler.sender.take();
    }


}

async fn process_orderbook(
    client: Arc<CoinbasePublicClient>,
    orderbook: Arc<RwLock<OrderBookL3>>,
    mut ob_event_rx: UnboundedReceiver<MarketEvent>,
) {
    // request snapshot for orderbook and deserialize into generic OrderBookL3Snapshot
    let product_id_str: String;
    let market: Market;
    {
        let reader = orderbook.read().await;
        product_id_str = format!("{}-{}", reader.market.instrument.base, reader.market.instrument.quote);
        market = reader.market.clone();
    }
    let snapshot_str = client
        .get_product_orderbook(&product_id_str, OBLevel::Level3)
        .await.unwrap();
    let snapshot: OrderBookL3Snapshot = serde_json::from_str::<CoinbaseOrderBookL3Snapshot>(&snapshot_str)
        .unwrap().into();

    // optionally, save the snapshot into a text file
    if SAVE_SNAPSHOT { let _ = save_orderbook_snapshot(&market, snapshot_str); }

    // load snapshot into orderbook
    {
        let mut writer  = orderbook.write().await;
        let snapshot_sequence = snapshot.last_update_id.clone();
        writer.process(
            OrderBookEvent::Snapshot(snapshot, snapshot_sequence)
        );
        info!("Loaded snapshot for {:?}", market);
    }

    // main event processing loop
    while let Some(event) = ob_event_rx.recv().await {
        match event.kind {
            OBEvent(orderbook_event) => {
                let mut writer = orderbook.write().await;
                writer.process(orderbook_event)
            }
            _ => {},
       }
    }

    info!("Orderbook processor for {:?} has finished", market);
}

async fn run_streams(
    subscriptions: Vec<Subscription>,
    mut stop_rx: oneshot::Receiver<()>,
    mut event_tx: UnboundedSender<MarketEvent>,
) {
    let streams = Streams::builder()
        .subscribe(subscriptions)
        .init().await.unwrap();

    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    loop {
        tokio::select! {
            _x = &mut stop_rx => {
                break
            },

            response = joined_stream.next() => {
                if let Some((exchange, market_event)) = response {
                    // pretty_print_trade(&market_event);
                    event_tx.send(market_event);
                }
            }
        }
    }
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

/// Send kill signal when hitting ctrl+c for graceful shutdown
async fn ctrl_c_watch(tx: oneshot::Sender<()>) {
    signal::ctrl_c().await.unwrap();
    tx.send(()).unwrap();
}

// enum OrderbookL3StreamState {
//     Snapshot,
//     Backfill,
//     Normal,
// }
//
// impl OrderbookL3StreamState {
//     pub fn iter() -> Iter<'static, OrderbookL3StreamState> {
//         static STATE_ORDER: [OrderbookL3StreamState; 3] = [
//             OrderbookL3StreamState::Snapshot,
//             OrderbookL3StreamState::Backfill,
//             OrderbookL3StreamState::Normal,
//         ];
//         STATE_ORDER.iter()
//     }
// }

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

fn save_orderbook_snapshot(market: &Market, snapshot: String) -> std::io::Result<()> {
    let mut path = env::current_dir()?;
    path.push(format!("/data/{}/", Utc::today().to_string()));
    fs::create_dir_all(&path)?;
    let filename = format!(
        "snapshot_{}_{}_{}.json",
        market.exchange,
        market.instrument,
        Utc::now().format("%Y%m%d_%H%M%S").to_string());
    path.push(filename);
    info!("Saving snapshot into {}", path.to_str().unwrap());
    let mut f = fs::File::create(path).expect("Unable to create snapshot file");
    f.write_all(snapshot.as_bytes()).expect("Unable to write snapshot file");
    Ok(())
}
