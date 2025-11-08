use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use cached::proc_macro::once;

#[cfg(feature = "monero")]
pub mod monero;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    BTC(String),
    #[cfg(feature = "monero")]
    XMR(String),
}

impl Address {
    pub fn try_parse(currency: &str, address: &str) -> Option<Self> {
        // TODO validate address
        match currency.to_lowercase().as_str() {
            "btc" => Some(Self::BTC(address.into())),
            #[cfg(feature = "monero")]
            "xmr" => Some(Self::XMR(address.into())),
            _ => None,
        }
    }
}

/// Lookup the current USD value of the given currency.
#[once(time = "3600", result = true)]
pub async fn lookup(symbol: &str) -> Result<f64> {
    let mapping: HashMap<String, f64> = reqwest::get(format!(
        "https://min-api.cryptocompare.com/data/price?fsym=USD&tsyms={symbol}"
    ))
    .await?
    .json()
    .await?;

    Ok(*mapping.get(symbol).unwrap())
}
