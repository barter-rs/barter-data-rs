use super::subscription::SubscriptionMap;
use barter_integration::{
    error::SocketError,
    protocol::{StreamParser, websocket::{WebSocket, WebSocketParser}},
    Validator,
};
use std::{
    time::Duration,
};
use futures::stream::StreamExt;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait SubscriptionValidator {
    type Parser: StreamParser;

    async fn validate<Kind, SubResponse>(
        map: SubscriptionMap<Kind>,
        // stream: &mut <<Self as SubscriptionValidator>::Parser as StreamParser>::Stream,
        websocket: &mut WebSocket,
        expected_responses: usize,
    ) -> Result<SubscriptionMap<Kind>, SocketError>
        where
            Kind: Send,
            SubResponse: Validator + DeserializeOwned;

    fn subscription_timeout() -> Duration { Duration::from_secs(10) }
}

pub struct WebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for WebSocketSubValidator {
    type Parser = WebSocketParser;

    async fn validate<Kind, SubResponse>(map: SubscriptionMap<Kind>, websocket: &mut WebSocket, expected_responses: usize) -> Result<SubscriptionMap<Kind>, SocketError>
    where
        Kind: Send,
        SubResponse: Validator + DeserializeOwned
    {
        // Establish time limit in which we expect to validate all the Subscriptions
        let timeout = Self::subscription_timeout();

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                break Ok(map);
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {:?}", timeout)
                    ))
                },
                // Parse incoming messages and determine subscription outcomes
                message = websocket.next() => {
                    let response = match message {
                        Some(response) => response,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    };

                    match Self::Parser::parse::<SubResponse>(response) {
                        Some(Ok(response)) => match response.validate() {
                            // Subscription success
                            Ok(_) => { success_responses += 1; }

                            // Subscription failure
                            Err(err) => break Err(err)
                        }
                        Some(Err(SocketError::Terminated(close_frame))) => {
                            break Err(SocketError::Subscribe(
                                format!("received WebSocket CloseFrame: {close_frame}")
                            ))
                        }
                        _ => {
                            // Pings, Pongs, Frames, and already active Subscriptions events
                            continue
                        }
                    }
                }
            }
        }
    }
}