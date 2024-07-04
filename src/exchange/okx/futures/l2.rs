use crate::{
    error::DataError,
    exchange::okx::book::{
        l2::{OkxFuturesOrderBookDelta, OkxOrderBookAction},
        OkxLevel,
    },
    subscription::book::{OrderBook, OrderBookSide},
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
};
use async_trait::async_trait;
use barter_integration::{
    model::{instrument::Instrument, Side},
    protocol::websocket::WsMessage,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxFuturesBookUpdater {
    /// The smallest possible sequence ID value is 0, except in snapshot messages where the prevSeqId is always -1.
    pub prev_seq_id: i64,
}

impl OkxFuturesBookUpdater {
    pub fn new() -> Self {
        Self { prev_seq_id: 0 }
    }
}

#[async_trait]
impl OrderBookUpdater for OkxFuturesBookUpdater {
    type OrderBook = OrderBook;
    type Update = OkxFuturesOrderBookDelta;

    async fn init<Exchange, Kind>(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Instrument, Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        // Initial orderbook is empty since the snapshot comes from the first message in the
        // websocket
        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(),
            book: OrderBook {
                last_update_time: Utc::now(),
                bids: OrderBookSide::new(Side::Buy, Vec::<OkxLevel>::new()),
                asks: OrderBookSide::new(Side::Sell, Vec::<OkxLevel>::new()),
            },
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        for data in update.data {
            let seq_id = data.seq_id;

            match update.action {
                // The first message in the websocket stream will be a snapshot
                OkxOrderBookAction::SNAPSHOT => {
                    *book = OrderBook::from(data);
                }
                // All consecutive messages will be deltas
                OkxOrderBookAction::UPDATE => {
                    // Update OrderBook metadata & Levels:
                    book.last_update_time = data.time;
                    book.bids.upsert(data.bids);
                    book.asks.upsert(data.asks);

                    // Missed a message
                    if self.prev_seq_id != data.prev_seq_id {
                        return Err(DataError::InvalidSequence {
                            prev_last_update_id: self.prev_seq_id as u64,
                            first_update_id: data.seq_id as u64,
                        });
                    }

                    // If there are no updates to the depth for an extended period, OKX will send a message
                    // with 'asks': [], 'bids': [] to inform users that the connection is still active.
                    // `seqId` is the same as the last sent message and `prevSeqId` equals to `seqId`
                    //
                    // See docs: <https://www.okx.com/docs-v5/en/#order-book-trading-market-data-ws-order-book-channel>
                    if data.seq_id == data.prev_seq_id {
                        return Ok(None);
                    }

                    //
                    // Verify checksum
                    // TODO: Remove this + crc32 dependency
                    //
                    book.bids.sort();
                    book.asks.sort();

                    // Calc checksum
                    let expected_checksum = data.checksum;
                    let checksum = book
                        .bids
                        .levels
                        .iter()
                        .take(25)
                        .zip(book.asks.levels.iter().take(25))
                        .map(|(bid, ask)| {
                            format!("{}:{}:{}:{}", bid.price, bid.amount, ask.price, ask.amount)
                        })
                        .collect::<Vec<_>>()
                        .join(":");

                    println!("{checksum}");
                    let mut hasher = crc32fast::Hasher::new();
                    hasher.update(checksum.as_bytes());
                    let checksum = hasher.finalize() as i32;

                    println!("\n\n\n -----------------");
                    println!("expected {expected_checksum} actual {checksum}");
                    println!("\n\n\n -----------------");
                }
            };

            // Update OrderBookUpdater metadata
            self.prev_seq_id = seq_id;
        }

        Ok(Some(book.snapshot()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use crate::exchange::okx::book::l2::OkxOrderBookData;
        use barter_integration::model::SubscriptionId;
        use chrono::DateTime;

        #[test]
        fn test_okx_futures_order_book_l2_deltas() {
            let input = r#"
            {
              "arg": {
                "channel": "books",
                "instId": "BTC-USDT"
              },
              "action": "update",
              "data": [
                {
                  "asks": [
                    ["8476.98", "415", "0", "13"],
                  ],
                  "bids": [
                    ["8476.97", "256", "0", "12"],
                  ],
                  "ts": "1597026383085",
                  "checksum": -855196043,
                  "prevSeqId": 123456,
                  "seqId": 123457
                }
              ]
            }
            "#;

            assert_eq!(
                serde_json::from_str::<OkxFuturesOrderBookDelta>(input).unwrap(),
                OkxFuturesOrderBookDelta {
                    subscription_id: SubscriptionId::from("TODO"),
                    action: OkxOrderBookAction::UPDATE,
                    data: vec![OkxOrderBookData {
                        time: DateTime::<Utc>::from_timestamp_millis(1597026383085).unwrap(),
                        asks: vec![OkxLevel {
                            price: 8476.98,
                            amount: 415.0
                        }],
                        bids: vec![OkxLevel {
                            price: 8476.97,
                            amount: 256.0
                        }],
                        checksum: -855196043,
                        prev_seq_id: 123456,
                        seq_id: 123457
                    }]
                }
            )
        }
    }

    mod okx_futures_book_updater {
        use super::*;

        #[test]
        fn test_validate_update() {
            todo!()
        }
    }
}
