# Barter-Data
A high-performance WebSocket integration library for streaming public data from leading cryptocurrency 
exchanges - batteries included. It is:
* **Normalised**: Barter-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
* **Real-Time**: Barter-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.
* **Easy**: Barter-Data's simple ExchangeClient interface allows for easy & quick setup.
* **Extensible**: Barter-Data is highly extensible, and therefore easy to contribute to via new integrations!

**Note: Barter-Data is recommended for use with the [`Barter`].**

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
[API Documentation]: https://docs.rs/barter-data/latest/barter_data
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter-Data is a high-performance WebSocket integration library for streaming public data from leading cryptocurrency 
exchanges. It presents an easy to use, extensible, interface that can deliver normalised exchange data in real-time.
At a high level, it provides a few major components:
* ConnectionHandler that manages the WebSocket connection (ping-pongs, re-connections, rate-limiting) and actions subscription
  requests on behalf of an Exchange Client implementation (eg/ Binance Exchange Client).
* Unified ExchangeClient trait that enables easy extensibility, ease of use, and the delivery of a normalised data model
  to downstream consumers.

## Example
Binance tick-by-tick Trade consumer with Barter-Data.

```rust,no_run
use barter_data::client::binance::Binance;
use barter_data::client::ClientConfig;
use barter_data::ExchangeClient;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise new Binance Exchange Client
    let mut binance = Binance::init(ClientConfig {
        rate_limit_per_minute: 300
    }).await?;
    
    // Subscribe & consume normalised Trade stream
    let mut trade_stream = binance
        .consume_trades(String::from("btcusdt"))
        .await?;
    
    // Loop over arriving Trades
    while let Some(trade) = trade_stream.next().await {
        // Do something with normalised Trade
        println!("{:?}", trade);
    }
}
```
**For a larger, "real world" example, see the [`Barter`] repository.**

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be 
happy to help to [Chat] and try answer your question via Discord. 

## Contributing
:tada: Thanks for your help in improving the barter ecosystem! Please do get in touch on the discord to discuss 
development, new features, and the future roadmap. 

## Related Projects
In addition to the Barter-Data crate, the Barter project also maintains:
* [`Barter`]: High-performance, extensible & modular trading components with batteries-included. Contains a 
pre-built trading Engine that can serve as a live-trading or backtesting system.

## Roadmap
* Extend the existing integrations scope to include more endpoints.
* Implement integrations for more exchanges.

## Licence
This project is licensed under the [MIT license].

[MIT license]: https://github.com/tokio-rs/tokio/blob/master/LICENSE

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.
