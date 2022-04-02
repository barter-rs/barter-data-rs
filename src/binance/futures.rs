use std::collections::HashMap;
use std::ops::DerefMut;
use serde_json::json;
use barter_integration::Instrument;
use barter_integration::socket::error::SocketError;
use barter_integration::socket::protocol::websocket::ExchangeWebSocket;
use barter_integration::socket::Transformer;
use crate::binance::BinanceMessage;
use crate::{ExchangeId, ExchangeTransformer, MarketEvent, StreamId, StreamIdentifier, Subscription};
use crate::model::{MarketData, Sequence, StreamKind, StreamMeta};
use serde::{Deserialize, Serialize};

// Todo: Can I simplify these ie/ remove generics or derive some generics from others
pub type BinanceFuturesStream = ExchangeWebSocket<BinanceFutures, BinanceMessage, MarketEvent>;

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFutures {
    pub streams: HashMap<StreamId, StreamMeta>
}

impl ExchangeTransformer for BinanceFutures {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFutures;
    const BASE_URL: &'static str = "wss://fstream.binance.com/ws";

    fn new() -> Self {
        Self { streams: HashMap::new() }
    }

    fn generate_subscriptions(&mut self, subscriptions: &[Subscription]) -> Vec<serde_json::Value> {
        // Map Barter Subscriptions to a vector of BinanceFutures StreamIds
        let channels = subscriptions
            .into_iter()
            .map(|subscription| {
                // Determine the BinanceFutures specific channel for this Subscription
                let stream_id = BinanceFutures::get_stream_id(subscription);

                // Add channel with the associated original Subscription to the internal HashMap
                self.streams
                    .insert(stream_id.clone(), StreamMeta::new(subscription.clone()));

                stream_id
            })
            .collect::<Vec<StreamId>>();

        // Construct BinanceFutures specific subscription message for all desired channels
        vec![json!({
            "method": "SUBSCRIBE",
            "params": channels,
            "id": 1
        })]
    }
}

impl Transformer<MarketEvent> for BinanceFutures {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Output vector to return (only ever 0 or 1 length)
        let mut output_iter = Vec::with_capacity(1);

        match input {
            BinanceMessage::Subscribed(sub_outcome) => {
                if sub_outcome.is_failure() {
                    output_iter.push(Err(SocketError::SubscribeError(
                        "received Binance subscription failure".to_string()
                    )))
                }
            }
            BinanceMessage::Trade(trade) => {
                let (instrument, sequence) = match self.get_stream_meta(&trade.to_stream_id()) {
                    Ok(stream_meta) => stream_meta,
                    Err(err) => {
                        output_iter.push(Err(err));
                        return output_iter;
                    },
                };

                output_iter.push(Ok(MarketEvent::new(
                    sequence,
                    MarketData::from((BinanceFutures::EXCHANGE, instrument, trade))
                )))
            }
        };

        output_iter
    }
}

impl BinanceFutures {
    fn get_stream_id(sub: &Subscription) -> StreamId {
        match sub.kind {
            StreamKind::Trades => {
                StreamId(format!("{}{}@aggTrade", sub.instrument.base, sub.instrument.quote))
            }
            _ => panic!("unsupported")
        }
    }

    fn get_stream_meta(&mut self, stream_id: &StreamId) -> Result<(Instrument, Sequence), SocketError> {
        self.streams
            .get_mut(stream_id)
            .map(|stream_meta| {
                // Increment the Sequence number associated with this StreamId
                let sequence = stream_meta.sequence;
                *stream_meta.sequence.deref_mut() += 1;

                (stream_meta.subscription.instrument.clone(), sequence)
            })
            .ok_or_else(|| SocketError::Unidentifiable(stream_id.0.clone()))
    }
}