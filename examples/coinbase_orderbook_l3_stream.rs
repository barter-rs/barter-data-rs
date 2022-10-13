#![allow(dead_code)]

use barter_data::{
    builder::Streams,
    ExchangeId,
    model::{MarketEvent, SubKind},
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;
use tokio::{signal, sync::oneshot};
use tokio::sync::mpsc;
use barter_data::model::{DataKind, Subscription};
use barter_data::orderbook::OrderbookEvent;

enum StreamState {
    Snapshot,
    Backfill,
    Normal,
}

async fn run_streams(subscriptions: Vec<Subscription>, mut stop_rx: oneshot::Receiver<()>) {

    let streams = Streams::builder()
        .subscribe(subscriptions)
        .init().await.unwrap();

    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    // let mut last_seq: Option<u64> = None;
    let mut last_seq: u64;

    loop {
        tokio::select! {
            _x = &mut stop_rx => break,

            response = joined_stream.next() => {
                if let Some((exchange, market_event)) = response {

                    println!("{:?}", market_event);

                    // let data: DataKind = market_event.kind;
                    // let curr_seq: u64 = data.sequence().to_owned();

                    // // check missed sequences
                    // if last_seq.is_some() && curr_seq != last_seq.unwrap() + 1 {
                    //     print!("missed sequence: {} - ", last_seq.unwrap());
                    //     println!("{:?}", data);
                    // }
                    // last_seq = Some(curr_seq);

                }
            }
        }
    }
}

async fn orderbook_handler() {
    // let orderbook = OrderBookL3::new();
}

/// Send kill signal when hitting ctrl+c for graceful shutdown
async fn ctrl_c_watch(tx: oneshot::Sender<()>) {
    signal::ctrl_c().await.unwrap();
    tx.send(()).unwrap();
}

#[tokio::main]
async fn main() {

    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    let (order_event_tx, order_event_rx)
        = mpsc::unbounded_channel::<OrderbookEvent>();

    let subscriptions: Vec<Subscription> = vec![
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::OrderBookL3).into()
    ];

    let _stream_thread = tokio::spawn(run_streams(subscriptions, stop_rx));
    let stop_thread = tokio::spawn(ctrl_c_watch(stop_tx));

    stop_thread.await.unwrap();

}
