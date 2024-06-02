use anyhow::Result;
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    Json,
};
use cached::proc_macro::once;
use monero_rpc::{RpcClientBuilder, WalletClient};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::{cli::serve::ServeArgs, AppState, CommandLine};

#[derive(Clone, Debug)]
pub struct MoneroState {
    wallet: WalletClient,
    wallet_address: String,
}

impl MoneroState {
    pub async fn new(args: &ServeArgs) -> anyhow::Result<Self> {
        debug!("Connecting to wallet RPC");
        let wallet = RpcClientBuilder::new()
            .rpc_authentication(monero_rpc::RpcAuthentication::Credentials {
                username: "monero".to_string(),
                password: "".to_string(),
            })
            .build("http://127.0.0.1:1234")?
            .wallet();

        info!(
            block_height = wallet.get_height().await?,
            "Connected to wallet RPC"
        );

        let address = wallet.get_address(0, None).await?;

        Ok(Self {
            wallet,
            wallet_address: address.address.public_spend.to_string(),
        })
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self.wallet.get_balance(0, None).await?;
        Ok(balance.unlocked_balance.as_pico())
    }
}
