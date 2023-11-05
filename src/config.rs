use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct AppConfig {

    /// HTTP URL of the repository we're managing
    repo_url: String,

    payout: PayoutConfig,

    wallet: WalletConfig,

    github: Option<GithubConfig>,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        config::Config::builder().add_source(config::Environment::with_prefix("TURBINE").try_parsing(true).separator("_")).build()?.try_deserialize()?
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct PayoutConfig {
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct WalletConfig {
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct GithubConfig {
    /// Github API token
    token: String,
}
