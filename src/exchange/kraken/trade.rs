use crate::{
    event::{Market, MarketIter},
    exchange::ExchangeId,
    subscription::trade::PublicTrade,
    Identifier,
};
use barter_integration::{
    de::{datetime_utc_from_epoch_duration, extract_next},
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Collection of [`KrakenTrade`] items with an associated [`SubscriptionId`] (eg/ "trade|XBT/USD").
///
/// See [`KrakenMessage`](super::message::KrakenMessage) for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrades {
    pub subscription_id: SubscriptionId,
    pub trades: Vec<KrakenTrade>,
}

/// [`Kraken`](super::Kraken) trade.
///
/// See [`KrakenMessage`](super::message::KrakenMessage) for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrade {
    pub price: f64,
    #[serde(rename = "quantity")]
    pub amount: f64,
    pub time: DateTime<Utc>,
    pub side: Side,
}

impl Identifier<Option<SubscriptionId>> for KrakenTrades {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// Todo:
fn custom_kraken_trade_id(trade: &KrakenTrade) -> String {
    format!(
        "{}_{}_{}_{}",
        trade.time.timestamp_nanos(),
        trade.side,
        trade.price,
        trade.amount
    )
}

impl From<(ExchangeId, Instrument, KrakenTrades)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trades): (ExchangeId, Instrument, KrakenTrades)) -> Self {
        trades
            .trades
            .into_iter()
            .map(|trade| {
                Ok(Market {
                    exchange_time: trade.time,
                    received_time: Utc::now(),
                    exchange: Exchange::from(exchange_id),
                    instrument: instrument.clone(),
                    event: PublicTrade {
                        id: custom_kraken_trade_id(&trade),
                        price: trade.price,
                        amount: trade.amount,
                        side: trade.side,
                    },
                })
            })
            .collect()
    }
}

impl<'de> serde::de::Deserialize<'de> for KrakenTrades {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTrades;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTrades struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenTrades Sequence Format:
                // [channelID, [[price, volume, time, side, orderType, misc]], channelName, pair]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract Vec<KrakenTrade>
                let trades = extract_next(&mut seq, "Vec<KrakenTrade>")?;

                // Extract channelName (eg/ "trade") & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "trade|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("trade|{pair}")))?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTrades {
                    subscription_id,
                    trades,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrades
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> serde::de::Deserialize<'de> for KrakenTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTrade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTrade struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenTrade Sequence Format:
                // [price, volume, time, side, orderType, misc]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract String price & parse to f64
                let price = extract_next::<SeqAccessor, String>(&mut seq, "price")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;

                // Extract String amount & parse to f64
                let amount = extract_next::<SeqAccessor, String>(&mut seq, "quantity")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;

                // Extract String price, parse to f64, map to DateTime<Utc>
                let time = extract_next::<SeqAccessor, String>(&mut seq, "time")?
                    .parse()
                    .map(|time| {
                        datetime_utc_from_epoch_duration(std::time::Duration::from_secs_f64(time))
                    })
                    .map_err(serde::de::Error::custom)?;

                // Extract Side
                let side: Side = extract_next(&mut seq, "side")?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTrade {
                    price,
                    amount,
                    time,
                    side,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrade
        deserializer.deserialize_seq(SeqVisitor)
    }
}
