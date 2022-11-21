// use barter_integration::error::SocketError;
// use barter_integration::protocol::websocket::{connect, WebSocket};
// use futures::Stream;
// use crate::model::subscription::{Subscription, SubscriptionIds};
// use async_trait::async_trait;
//
//
// struct SubscriptionMeta<Message> {
//     subscription_map: SubscriptionIds,
//     expected_response: usize,
//     subscriptions: Vec<Message>,
// }
//
// #[async_trait]
// pub trait Connector {
//     type Stream: Stream;
//     type Error;
//     async fn connect<R>(&self, request: R) -> Result<Self::Stream, Self::Error>
//     where
//         R: IntoClientRequest;
// }
//
// struct WsConnector;
// impl Connector for WsConnector {
//     type Stream = WebSocket;
//     type Error = SocketError;
//     async fn connect<R>(&self, request: R) -> Result<Self::Stream, Self::Error> where R: IntoClientRequest {
//         connect(request)
//     }
// }
//
// pub trait SubscriptionMapper {
//     type Message;
//     fn build_subscription_meta(subscriptions: &[Subscription]) -> Result<SubscriptionMeta<Self::Message>, SocketError>;
// }
//
// #[async_trait]
// pub trait SubscriptionValidator {
//     type Stream: Stream;
//     type Error;
//     async fn validate(
//         subscription_map: SubscriptionIds,
//         stream: &mut Self::Stream,
//         expected_response: usize,
//     ) -> Result<SubscriptionIds, Self::Error>;
// }
//
// struct SubValidator {
//     // Todo: Break down the validation algorithm to account for eg/ checking response directly & binary deserialisation
// }
//
//
// struct Subscriber<Connector, SubTransformer, Validator> {
//     connector: Connector,
//     transformer: SubTransformer,
//     validator: Validator,
// }
//
// impl<Connector, SubMapper, SubValidator> Subscriber<Connector, SubMapper, SubValidator>
// where
//     Connector: Connector,
//     SubscriptionMeta: TryFrom<SubMapper>,