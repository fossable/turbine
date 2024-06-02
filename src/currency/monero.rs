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
use std::{
    process::{Child, Command},
    sync::Arc,
};
use tracing::{debug, info, instrument};

use crate::{cli::serve::ServeArgs, AppState, CommandLine};

#[derive(Clone, Debug)]
pub struct MoneroState {
    wallet: WalletClient,
    wallet_process: Option<Arc<Child>>,
    pub wallet_address: String,
}

impl Drop for MoneroState {
    fn drop(&mut self) {
        if let Some(process) = self.wallet_process.as_mut() {
            debug!("Stopping RPC wallet daemon");
            process.kill().unwrap_or_default();
        }
    }
}

impl MoneroState {
    pub async fn new(args: &ServeArgs) -> anyhow::Result<Self> {
        // Spawn new wallet RPC process
        let rpc_port = 9999;
        let rpc_password = "1234";
        let wallet_process = Command::new("monero-wallet-rpc")
            .arg("--rpc-bind-port")
            .arg(format!("{}", rpc_port))
            .arg(if args.stagenet {
                "--stagenet"
            } else if args.testnet {
                "--testnet"
            } else {
                "--mainnet"
            })
            .arg("--password")
            .arg(rpc_password)
            .arg("--wallet-file")
            .arg("/wallet")
            .spawn()?;

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

        Ok(Self {
            wallet,
            wallet_process,
            wallet_address: wallet
                .get_address(0, None)
                .await?
                .address
                .public_spend
                .to_string(),
        })
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self.wallet.get_balance(0, None).await?;
        Ok(balance.unlocked_balance.as_pico())
    }
}
