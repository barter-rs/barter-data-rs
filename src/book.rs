use barter_integration::ExchangeStream;
use barter_integration::protocol::http::rest::client::RestClient;
use barter_integration::protocol::websocket::{WebSocketParser, WsStream};

pub type ExchangeWsStream<Exchange, Event> = ExchangeStream<WebSocketParser, WsStream, Exchange, Event>;

struct BookStream<Exchange, Event> {
    exchange_stream: ExchangeWsStream<Exchange, Event>,
    rest_client: RestClient<>
}