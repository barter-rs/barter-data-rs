use super::Okx;
use crate::{subscription::Subscription, Identifier};
use barter_integration::model::instrument::{
    kind::{InstrumentKind, OptionKind},
    Instrument,
};
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Utc,
};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Okx`](super::Okx) market that can be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMarket(pub String);

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, Kind> {
    fn id(&self) -> OkxMarket {
        use InstrumentKind::*;
        let Instrument { base, quote, kind } = &self.instrument;

        OkxMarket(match kind {
            Spot => format!("{base}-{quote}").to_uppercase(),
            Future(future) => {
                format!("{base}-{quote}-{}", format_expiry(future.expiry)).to_uppercase()
            }
            Perpetual => format!("{base}-{quote}-SWAP").to_uppercase(),
            Option(option) => format!(
                "{base}-{quote}-{}-{}-{}",
                format_expiry(option.expiry),
                option.strike,
                match option.kind {
                    OptionKind::Call => "C",
                    OptionKind::Put => "P",
                },
            )
            .to_uppercase(),
        })
    }
}

impl AsRef<str> for OkxMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Format the expiry DateTime<Utc> to be Okx API compatible.
///
/// eg/ "230526" (26th of May 2023)
///
/// See docs: <https://www.okx.com/docs-v5/en/#rest-api-public-data-get-instruments>
fn format_expiry<'a>(expiry: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    expiry.date_naive().format("%g%m%d")
}
