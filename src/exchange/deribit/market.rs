use crate::exchange::deribit::Deribit;
use crate::instrument::MarketInstrumentData;
use crate::subscription::Subscription;
use crate::Identifier;
use barter_integration::model::instrument::kind::{
    FutureContract, InstrumentKind, OptionContract, OptionExercise, OptionKind,
};
use barter_integration::model::instrument::Instrument;
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Deribit`] market that can be subscribed to.
///
/// See docs: <https://docs.deribit.com/#subscriptions>
/// See docs: <<https://docs.deribit.com/#naming>>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitMarket(pub String);

impl<Kind> Identifier<DeribitMarket> for Subscription<Deribit, Instrument, Kind> {
    fn id(&self) -> DeribitMarket {
        let Instrument { base, quote, kind } = &self.instrument;

        DeribitMarket(match kind {
            InstrumentKind::Spot => {
                // Format: <BASE_SYMBOL>_<QUOTE_SYMBOL>
                format!("{base}_{quote}").to_uppercase()
            }
            InstrumentKind::Future(FutureContract { expiry }) => {
                assert_eq!(
                    quote.as_ref(),
                    "usd",
                    "Deribit Futures are always quoted in usd"
                );

                // Format: <BASE_SYMBOL>_<DMMMYY>
                format!("{base}-{}", expiry.date_naive().format("%-d%b%y")).to_uppercase()
            }
            InstrumentKind::Perpetual => {
                if quote.as_ref() == "usd" {
                    // Format: <BASE_SYMBOL>-PERPETUAL
                    format!("{base}-PERPETUAL").to_uppercase()
                } else {
                    // Format: <BASE_SYMBOL>_<QUOTE_SYMBOL>-PERPETUAL
                    format!("{base}_{quote}-PERPETUAL").to_uppercase()
                }
            }
            InstrumentKind::Option(OptionContract {
                kind,
                exercise,
                expiry,
                strike,
            }) => {
                assert_eq!(
                    *exercise,
                    OptionExercise::European,
                    "Deribit Options are always european exercise"
                );

                // Format: <BASE_SYMBOL>-<DMMMYY>-<STRIKE_IN_USD>-<OPTION_KIND_FIRST_LETTER>
                format!(
                    "{base}-{}-{strike}-{}",
                    expiry.date_naive().format("%-d%b%y"),
                    match kind {
                        OptionKind::Call => "C",
                        OptionKind::Put => "P",
                    }
                )
                .to_uppercase()
            }
        })
    }
}

impl<Kind> Identifier<DeribitMarket> for Subscription<Deribit, MarketInstrumentData, Kind> {
    fn id(&self) -> DeribitMarket {
        DeribitMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for DeribitMarket {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
