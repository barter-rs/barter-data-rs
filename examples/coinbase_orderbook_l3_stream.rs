use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind},
        orderbook::{OrderbookMap, OrderbookL3},
        DataKind,
    },
};
use barter_integration::model::{InstrumentKind, Market, Side};
use futures::StreamExt;
use tokio::{signal, sync::oneshot};
use tracing_subscriber;

// enum StreamState {
//     Snapshot,
//     Backfill,
//     Normal,
// }

async fn run_streams(subscriptions: Vec<Subscription>, mut stop_rx: oneshot::Receiver<()>) {

    // build orderbook for each OrderbookL3 subscription
    let mut orderbook_map = OrderbookMap::new();
    subscriptions
        .iter()
        .filter(|subscription| subscription.kind == SubKind::OrderBookL3Delta)
        .for_each(|subscription| {
            orderbook_map.insert(
                OrderbookL3::builder()
                    .market(Market::from(
                        (subscription.exchange.clone(), subscription.instrument.clone())
                    ))
                    .stats(false)
                    .last_n_events(5)
                    .outlier_filter_default()
                    .build()
                    .unwrap()
            )
        });

    let streams = Streams::builder()
        .subscribe(subscriptions)
        .init().await.unwrap();

    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    loop {
        tokio::select! {
            _x = &mut stop_rx => {
                for (.., orderbook) in orderbook_map.map {
                    // print some stats regarding the orderbook's operations
                    orderbook.print_info(true)
                }
                break
            },

            response = joined_stream.next() => {
                if let Some((_exchange, market_event)) = response {

                    pretty_print_trade(&market_event);

                    match market_event.kind {
                        DataKind::OrderBookEvent(ref event) => {
                            orderbook_map
                                .get_mut(&market_event.market())
                                .unwrap()
                                .process(event.clone())
                        },
                        _ => {}
                    }
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

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt().init();

    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    let subscriptions: Vec<Subscription> = vec![
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
        // (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade).into(),
        // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::OrderBookL3Delta).into(),
    ];

    let _stream_thread = tokio::spawn(run_streams(subscriptions, stop_rx));
    let stop_thread = tokio::spawn(ctrl_c_watch(stop_tx));

    stop_thread.await.unwrap();

}
