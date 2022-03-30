

// pub trait ExchangeClient {
//     const EXCHANGE_NAME: &'static str;
//     async fn consume_trades(&mut self, symbol: String, ) -> Result<UnboundedReceiver<Trade>, ClientError>;
//     async fn consume_candles(&mut self, symbol: String, interval: &str) -> Result<UnboundedReceiver<Candle>, ClientError>;
// }

use barter_integration::{
    public::{
        ExchangeId,
        model::{Subscription, MarketEvent}
    }
};
use std::collections::HashMap;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::StreamMap;
use tracing::{error, info};
use barter_integration::public::MarketStream;
use barter_integration::socket::error::SocketError;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Streams {
    pub streams: HashMap<ExchangeId, UnboundedReceiver<MarketEvent>>
}

impl Streams {
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    pub fn select(&mut self, exchange: ExchangeId) -> Option<UnboundedReceiver<MarketEvent>> {
        self.streams
            .remove(&exchange)
    }

    pub async fn join(mut self) -> StreamMap<ExchangeId, UnboundedReceiver<MarketEvent>> {
        self.streams
            .into_iter()
            .fold(
                StreamMap::new(),
                |mut map, (exchange, rx)| {
                    map.insert(exchange, rx);
                    map
                }
            )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct StreamBuilder {
    subscriptions: HashMap<ExchangeId, Vec<Subscription>>,
}

// Todo: Do I want to allow each Exchange's stream to generate one stream per subscription?
impl StreamBuilder {
    fn new() -> Self {
        Self { subscriptions: HashMap::new() }
    }

    pub fn subscribe<SubIter, Sub>(mut self, exchange: ExchangeId, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription>,
    {
        self.subscriptions
            .insert(exchange, subscriptions.into_iter().map(Sub::into).collect());

        self
    }

    pub async fn init(self) -> Result<Streams, SocketError> {
        // Determine total number of exchange subscriptions
        let num_exchanges = self.subscriptions.len();
        if num_exchanges == 0 {
            return Err(SocketError::SubscribeError("builder contains to Subscription to action".to_owned()))
        }

        // Construct Hashmap containing each Exchange's stream receiver
        let mut exchange_streams = HashMap::with_capacity(num_exchanges);

        for (exchange, subscriptions) in self.subscriptions {

            // Create channel for this exchange stream
            let (exchange_tx, exchange_rx) = mpsc::unbounded_channel();

            match exchange {
                ExchangeId::BinanceFutures => {

                }
                not_supported => {
                    return Err(SocketError::SubscribeError(not_supported.to_string()))
                }
            }

            exchange_streams.insert(exchange, exchange_rx);
        }

        Ok(Streams {
            streams: exchange_streams
        })
    }
}

async fn consume<S>(
    exchange: ExchangeId,
    subscriptions: &[Subscription],
    exchange_tx: UnboundedSender<MarketEvent>)
    -> SocketError
where
    S: MarketStream,
{
    info!(
        exchange,
        subscriptions,
        "running MarketStream consumer loop with exponential backoff retry policy"
    );

    // Consumer loop retry parameters
    let mut attempt: u32 = 0;

    loop {
        // Increment retry parameters at the start of every iteration
        attempt += 1;
        info!(
            exchange,
            attempt,
            "attempting to initialise exchange MarketStream"
        );

        // Attempt to initialise MarketStream: if it fails on first attempt return SocketError
        let mut stream = match S::init(subscriptions).await {
            Ok(stream) => {
                info!(exchange, attempt, "successfully initialised exchange MarketStream");
                attempt = 0;
                stream
            }
            Err(err) => {
                error!(
                    exchange,
                    attempt,
                    error = &*format!("{err:}"),
                    "failed to initialise exchange MarketStream"
                );

                // Exit function if MarketStream::init failed the first attempt, else retry
                if attempt == 1 {
                    return err
                } else {
                    continue
                }
            }
        };

        // Consume Result<IntoIter<Item = MarketEvents>, SocketError> from Stream
        while let Some(event) = stream.next().await {



        }
    }
}




