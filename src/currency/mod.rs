#[cfg(feature = "monero")]
pub mod monero;

#[derive(Clone, Debug)]
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
