use super::SubKind;
use barter_integration::model::Side;
use barter_macro::{DeSubKind, SerSubKind};
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields [`PublicTrade`]
/// [`Market`](crate::model::Market) events.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, DeSubKind, SerSubKind)]
pub struct PublicTrades;

impl SubKind for PublicTrades {
    type Event = PublicTrade;
}

/// Normalised Barter [`PublicTrade`] model.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PublicTrade {
    pub id: String,
    pub price: f64,
    pub amount: f64,
    pub side: Side,
}

#[test]
fn it_works() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Dummy<T> {
        kind: T,
    }

    let input = r#"
        {
            "kind": "public_trades"
        }
    "#;

    let actual = serde_json::from_str::<Dummy<PublicTrades>>(input).unwrap();

    println!("{actual:?}");

    assert_eq!(actual, Dummy { kind: PublicTrades });

    let actual = serde_json::to_string(&PublicTrades).unwrap();
    println!("{actual}");
}
