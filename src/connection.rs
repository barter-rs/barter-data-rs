use crate::error::ClientError;
use crate::{Identifier, StreamIdentifier, Subscription, WSStream};
use futures_util::SinkExt;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Interval};
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite::Message as WSMessage;

/// Type alias to communicate a stream's unique String identifier that can be used to route messages
/// from the [ConnectionHandler] to the relevant downstream consumer.
pub type StreamRoutingId = String;

/// Manages all connection related actions. This includes maintaining the WebSocket connection;
/// re-connections; actioning [Subscription] requests received from an ExchangeClient; consuming
/// incoming exchange messages from the WebSocket connection and routing them to the appropriate
/// downstream consumer.
pub struct ConnectionHandler<Message, Sub> {
    /// Rate limit interval in the form of a Future, awaiting it takes the [Duration] of the [Interval].
    pub rate_limit: Interval,
    /// An established [WSStream] connection that all ExchangeClient <--> ConnectionHandler
    /// communication goes through.
    pub ws_conn: WSStream,
    /// [Subscription] request channel receiver. Receives a tuple of [Subscription] and a data channel
    /// transmitter. This data channel transmitter is used to route messages relating to a particular
    /// [Subscription] back to the subscriber via the ExchangeClient implementation.
    pub subscription_rx: mpsc::Receiver<(Sub, mpsc::UnboundedSender<Message>)>,
    /// Map containing the data channel transmitter for every [Subscription] actioned. The map's
    /// [StreamRoutingId] key is used to identify which data channel to transmit an incoming
    /// exchange message to.
    pub exchange_data_txs: HashMap<StreamRoutingId, mpsc::UnboundedSender<Message>>,
}

impl<Message, Sub> ConnectionHandler<Message, Sub>
where
    Sub: Debug + Subscription + StreamIdentifier + Serialize + Send + Sync,
    Message: Debug + StreamIdentifier + DeserializeOwned + Send + Sync,
{
    /// Constructs a new [ConnectionHandler] instance using the [WSStream] connection provided.
    pub fn new(
        rate_limit_per_minute: u64,
        ws_conn: WSStream,
        subscription_rx: mpsc::Receiver<(Sub, mpsc::UnboundedSender<Message>)>,
    ) -> Self {
        Self {
            rate_limit: calculate_rate_limit_interval(rate_limit_per_minute),
            ws_conn,
            subscription_rx,
            exchange_data_txs: Default::default(),
        }
    }

    /// Consumes two types of incoming messages [Subscription] requests received from an
    /// ExchangeClient implementor instance, and also the data received from an exchange as a
    /// result of a [Subscription]. This function handles the actioning of [Subscription] requests,
    /// and routes the exchange data to the associated downstream subscriber.
    pub async fn manage(mut self) {
        loop {
            // Consume incoming messages:
            // 1) Subscription requests from ExchangeClient
            // 2) Incoming exchange data (trades, OrderBook updates, etc)

            tokio::select! {
                // Action incoming subscription requests from ExchangeClients
                Some((sub_request, data_tx)) = self.subscription_rx.recv() => {
                    self = self.action_subscription_request(sub_request, data_tx).await;
                }

                // Route incoming exchange data to the associated downstream subscriber
                Some(ws_message_result) = self.ws_conn.next() => {
                    // Rate limit consumption from the exchange via the WebSocket connection
                    self.rate_limit.tick().await;

                    // Handle WebSocket message Result
                    let ws_message = match ws_message_result {
                        Ok(ws_message) => ws_message,
                        Err(err) => {
                            warn!("Skipping message Result due to unexpected error: {:?}", err);
                            continue
                        },
                    };

                    // Handle WebSocket message variant, parsing a Message if present
                    let exchange_message = match ws_message {
                        WSMessage::Text(text) => {
                            match serde_json::from_str::<Message>(&*text.clone()) {
                                Ok(message) => message,
                                Err(err) => {
                                    error!("Failure to deserialise message: {:?} due to error: {:?}", text, err);
                                    continue;
                                },
                                }
                        },
                        WSMessage::Binary(binary) => {
                            warn!("Received unexpected binary message: {:?}", binary);
                            continue;
                        },
                        WSMessage::Close(close_frame) => {
                            info!("WebSocket connection closed with final frame: {:?}", close_frame);
                            break;
                        }
                        _ => continue,
                    };

                    // Determine StreamRoutingId associated with the Message
                    let routing_id = match exchange_message.get_stream_id() {
                        Identifier::Yes(routing_id) => routing_id,
                        Identifier::No => {
                            debug!("Ignoring received message due to no route: {:?}", exchange_message);
                            continue
                        },
                    };

                    // Retrieve data transmitter associated with the StreamRoutineId
                    let data_tx = self.retrieve_relevant_data_transmitter(&routing_id);

                    // Route Message to associated downstream subscriber
                    if data_tx.send(exchange_message).is_err() {
                        info!("Receiver for: {:?} has been dropped - closing stream", &routing_id);
                        self.exchange_data_txs.remove_entry(&routing_id);
                        continue;
                    }
                }
            }
        }
    }

    /// Action a [Subscription] request received from an ExchangeClient. An exchange data
    /// transmitter is inserted into the exchange_data_txs map upon subscribing, this is used by
    /// the [ConnectionHandler] to route incoming exchange messages to the associated downstream
    /// consumers.
    async fn action_subscription_request(
        mut self,
        sub_request: Sub,
        data_tx: mpsc::UnboundedSender<Message>,
    ) -> Self {
        info!(
            "Received Subscription request from ExchangeClient: {:?}",
            sub_request
        );

        // Identify StreamRoutingId of the Subscription
        let routing_id = match sub_request.get_stream_id() {
            Identifier::Yes(routing_id) => routing_id,
            Identifier::No => {
                warn!(
                    "Ignoring subscription request due to a non-identifiable routing_id: {:?}",
                    sub_request
                );
                return self;
            }
        };

        // Subscribe to stream via the WebSocket connection
        match self.subscribe(sub_request).await {
            Ok(_) => {
                // Add entry to the exchange_data_txs map
                self.exchange_data_txs.insert(routing_id.clone(), data_tx);
            }
            Err(err) => {
                warn!(
                    "Failed to subscribe to stream: {:?} due to error: {:?}",
                    routing_id, err
                )
            }
        }

        self
    }

    /// Subscribe asynchronously to a WebSocket data stream using the [Subscription] provided.
    pub async fn subscribe(&mut self, subscription: Sub) -> Result<(), ClientError> {
        self.ws_conn
            .send(WSMessage::text(subscription.as_text()?))
            .await
            .map_err(|write_err| ClientError::WebSocketWrite(write_err))?;
        Ok(())
    }

    /// Retrieves the data transmitter associated with a [StreamRoutingId] from the
    /// [ConnectionHandler]'s exchange_data_tx map.
    fn retrieve_relevant_data_transmitter(
        &mut self,
        routing_id: &String,
    ) -> &mut mpsc::UnboundedSender<Message> {
        self.exchange_data_txs.get_mut(routing_id).expect(
            format!(
                "Message with StreamRoutingId: {:?} has been received \
                                    without a relevant exchange_data_tx in the map to route \
                                    it to",
                routing_id
            )
            .as_str(),
        )
    }
}

fn calculate_rate_limit_interval(rate_limit_per_minute: u64) -> Interval {
    let rate_limit_per_second = (rate_limit_per_minute as f64 / 60.0) as f64;
    interval(Duration::from_secs_f64(rate_limit_per_second))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::binance::{BinanceMessage, BinanceSub};
    use crate::connect;

    // Binance Connection & Subscription
    async fn gen_binance_conn() -> WSStream {
        connect(&String::from("wss://stream.binance.com:9443/ws"))
            .await
            .unwrap()
    }
    fn gen_valid_binance_sub() -> BinanceSub {
        BinanceSub::new("@depth20@100ms".to_string(), "ethbtc".to_string())
    }

    // Bitstamp Connection & Subscription
    async fn gen_bitstamp_conn() -> WSStream {
        connect(&String::from("wss://ws.bitstamp.net/"))
            .await
            .unwrap()
    }
    // fn gen_valid_bitstamp_sub() -> BitstampSub {
    //     BitstampSub::new("order_book_".to_string(), "ethbtc".to_string())
    // }

    #[tokio::test]
    async fn test_calculate_rate_limit_interval() {
        struct TestCase {
            input_limit_per_min: u64,
            output_limit_duration: Duration,
        }

        let test_cases = vec![
            TestCase {
                // Test case 0:
                input_limit_per_min: 33,
                output_limit_duration: Duration::from_millis(550),
            },
            TestCase {
                // Test case 1:
                input_limit_per_min: 255,
                output_limit_duration: Duration::from_millis(4250),
            },
            TestCase {
                // Test case 2:
                input_limit_per_min: 5,
                output_limit_duration: Duration::from_secs_f64(1.0 / 12.0),
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            let actual_result = calculate_rate_limit_interval(test.input_limit_per_min);
            assert_eq!(
                test.output_limit_duration,
                actual_result.period(),
                "Test case: {:?}",
                index
            )
        }
    }

    #[tokio::test]
    async fn test_binance_subscribe() {
        struct TestCase {
            conn_handler: ConnectionHandler<BinanceMessage, BinanceSub>,
            input_sub: BinanceSub,
            expected_can_subscribe: bool,
        }

        let test_cases = vec![
            TestCase {
                // Test case 0: Valid Binance subscription
                conn_handler: ConnectionHandler::new(
                    4,
                    gen_binance_conn().await,
                    mpsc::channel(10).1,
                ),
                input_sub: gen_valid_binance_sub(),
                expected_can_subscribe: true,
            },
            TestCase {
                // Test case 1: Valid Binance subscription
                conn_handler: ConnectionHandler::new(
                    4,
                    gen_binance_conn().await,
                    mpsc::channel(10).1,
                ),
                input_sub: BinanceSub::new("invalid".to_string(), "invalid".to_string()),
                expected_can_subscribe: false,
            },
        ];

        for (index, mut test) in test_cases.into_iter().enumerate() {
            let actual_result = test.conn_handler.subscribe(test.input_sub).await;
            assert_eq!(
                test.expected_can_subscribe,
                actual_result.is_ok(),
                "Test case: {:?}",
                index
            );
        }
    }
}
