use crate::exchange::deribit::channel::DeribitChannel;
use crate::exchange::deribit::market::DeribitMarket;
use crate::exchange::subscription::ExchangeSub;
use crate::Identifier;
use barter_integration::model::SubscriptionId;
use serde::de::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitMessage<T> {
    pub params: DeribitParams<T>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitParams<T> {
    #[serde(
        rename = "channel",
        deserialize_with = "de_deribit_channel_field_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub data: T,
}

impl<T> Identifier<Option<SubscriptionId>> for DeribitMessage<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.params.subscription_id.clone())
    }
}

/// Deserialize an [`DeribitParams`] "channel" field as a Barter [`SubscriptionId`].
fn de_deribit_channel_field_as_subscription_id<'de, D>(
    deserializer: D,
) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let channel_field_value: &str = serde::de::Deserialize::deserialize(deserializer)?;
    parse_deribit_channel_field_as_subscription_id(channel_field_value).map_err(Error::custom)
}

fn parse_deribit_channel_field_as_subscription_id(value: &str) -> Result<SubscriptionId, String> {
    let mut tokens = value.split('.');

    let channel = tokens
        .next()
        .map(DeribitChannel)
        .ok_or_else(|| format!(
            "Deribit \"channel\" field .split('.') has unexpected format: missing first token: {value}")
        )?;

    let market = tokens
        .next()
        .map(|market| DeribitMarket(market.to_string()))
        .ok_or_else(|| format!(
            "Deribit \"channel\" field .split('.') has unexpected format: missing first token: {value}")
        )?;

    Ok(ExchangeSub::from((channel, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deribit_channel_field_as_subscription_id() {
        let valid_spot_channel = "trades.BTC_USDT";
        let actual = parse_deribit_channel_field_as_subscription_id(valid_spot_channel).unwrap();
        assert_eq!(actual, SubscriptionId("trades|BTC_USDT".to_string()));

        let valid_future_channel = "quote.BTC-24MAY24";
        let actual = parse_deribit_channel_field_as_subscription_id(valid_future_channel).unwrap();
        assert_eq!(actual, SubscriptionId("quote|BTC-24MAY24".to_string()));

        let valid_perpetual_channel = "trades.AVAX_USDC-PERPETUAL";
        let actual =
            parse_deribit_channel_field_as_subscription_id(valid_perpetual_channel).unwrap();
        assert_eq!(
            actual,
            SubscriptionId("trades|AVAX_USDC-PERPETUAL".to_string())
        );

        let valid_option_channel = "quote.BTC-24MAY24-58000-P";
        let actual = parse_deribit_channel_field_as_subscription_id(valid_option_channel).unwrap();
        assert_eq!(
            actual,
            SubscriptionId("quote|BTC-24MAY24-58000-P".to_string())
        );
    }
}
