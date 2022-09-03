# Barter-Data
A high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges - batteries included. It is:
* **Easy**: Barter-Data's simple StreamBuilder interface allows for easy & quick setup (see example below!).
* **Normalised**: Barter-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
* **Real-Time**: Barter-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.
* **Extensible**: Barter-Data is highly extensible, and therefore easy to contribute to with coding new integrations!

**See: [`Barter`], [`Barter-Integration`]**

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/barter-data.svg
[crates-url]: https://crates.io/crates/barter-data

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/blob/main/LICENCE

[actions-badge]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/badges/-/blob/main/pipeline.svg
[actions-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/commits/main

[discord-badge]: https://img.shields.io/discord/910237311332151317.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/wE7RqhnQMV

[API Documentation] |
[Chat]

[`Barter`]: https://crates.io/crates/barter
[`Barter-Integration`]: https://crates.io/crates/barter-integration
[API Documentation]: https://docs.rs/barter-data/latest/barter_data
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter-Data is a high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges. It presents an easy-to-use and extensible set of interfaces that can deliver normalised exchange data in real-time.

From a user perspective, it's major component is the `StreamBuilder` structures that assists in initialising an 
arbitrary number of exchange `MarketStreams` using an input `Subscription`. Simply build your dream set of 
`MarketStreams` and `Barter-Data` will do the rest!

## Example
`StreamBuilder` subscribing to various FuturePerpetual & Spot `MarketStreams` from `Ftx`, `Kraken` & `BinanceFuturesUsd`

```rust,no_run
use barter_data::{
    builder::Streams,
    model::{MarketEvent, SubKind},
    ExchangeId,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken & BinanceFuturesUsd
#[tokio::main]
async fn main() {
    // Initialise `PublicTrade` & `Candle``MarketStream` for `BinanceFuturesUsd`, `Ftx` & `Kraken`
    let streams = Streams::builder()
        .subscribe_exchange(
            ExchangeId::Ftx,
            [
                ("btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
                ("eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
                ("btc", "usdt", InstrumentKind::Spot, SubKind::Trade),
                ("eth", "usdt", InstrumentKind::Spot, SubKind::Trade),
            ],
        )
        .subscribe([
            (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Trade),
            (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            (ExchangeId::Kraken, "eth", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
        ])
        .init()
        .await
        .unwrap();

    // Join all exchange streams into a StreamMap
    // Note: Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    while let Some((exchange, event)) = joined_stream.next().await {
        println!("Exchange: {}, MarketEvent: {:?}", exchange, event);
    }
}
```

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be 
happy to help to [Chat] and try answer your question via Discord. 

## Contributing
Thanks for your help in improving the Barter ecosystem! Please do get in touch on the discord to discuss 
development, new features, and the future roadmap.
In order to integration a new exchange or endpoint, the following main traits will have to be implemented:
* **Subscriber**
* **ExchangeTransformer**

## Related Projects
In addition to the Barter-Data crate, the Barter project also maintains:
* [`Barter`]: High-performance, extensible & modular trading components with batteries-included. Contains a 
pre-built trading Engine that can serve as a live-trading or backtesting system.
* [`Barter-Integration`]: High-performance, low-level framework for composing flexible web integrations.

## Roadmap
* Extend the existing integrations scope to include more endpoints.
* Implement integrations for more exchanges.

## Licence
This project is licensed under the [MIT license].

[MIT license]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/blob/main/LICENSE

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Barter-Data by you, shall be licensed as MIT, without any additional
terms or conditions.
