use anyhow::Result;
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    Json,
};
use cached::proc_macro::once;
use monero_rpc::{RpcClientBuilder, WalletClient};
use serde::{Deserialize, Serialize};
use std::{
    process::{Child, Command},
    sync::Arc,
    time::Duration,
};
use tracing::{debug, info, instrument};

use crate::{cli::ServeArgs, CommandLine};

#[derive(Clone, Debug)]
pub struct MoneroState {
    wallet: WalletClient,
    wallet_process: Option<Arc<Child>>,
    pub wallet_address: String,
}

// impl Drop for MoneroState {
//     fn drop(&mut self) {
//         if let Some(process) = self.wallet_process.as_mut() {
//             debug!("Stopping RPC wallet daemon");
//             process.kill().unwrap_or_default();
//         }
//     }
// }

impl MoneroState {
    pub async fn new(args: &ServeArgs) -> anyhow::Result<Self> {
        // Spawn new wallet RPC process
        debug!("Spawning wallet RPC daemon");
        let wallet_process = Command::new("monero-wallet-rpc")
            .arg("--rpc-bind-port")
            .arg(format!("{}", args.monero_rpc_port))
            .arg(if args.stagenet {
                "--stagenet"
            } else if args.testnet {
                "--testnet"
            } else {
                // TODO hack
                "--non-interactive"
            })
            .arg("--password")
            .arg(&args.monero_wallet_password)
            .arg("--wallet-file")
            .arg(&args.monero_wallet)
            .arg("--daemon-address")
            .arg(&args.monero_daemon_address)
            .spawn()?;

        // Wait for the daemon to start
        std::thread::sleep(Duration::from_secs(20));

        debug!("Connecting to wallet RPC");

        // Read credentials from file
        let rpc_credentials =
            &std::fs::read_to_string(format!("monero-wallet-rpc.{}.login", &args.monero_rpc_port))?;
        let (rpc_username, rpc_password) = rpc_credentials.split_once(":").unwrap();

        let wallet = RpcClientBuilder::new()
            .rpc_authentication(monero_rpc::RpcAuthentication::Credentials {
                username: rpc_username.to_string(),
                password: rpc_password.to_string(),
            })
            .build(format!("http://127.0.0.1:{}", args.monero_rpc_port))?
            .wallet();

        info!(
            block_height = wallet.get_height().await?,
            "Connected to wallet RPC"
        );

        Ok(Self {
            wallet_process: Some(Arc::new(wallet_process)),
            wallet_address: wallet.get_address(0, None).await?.address.to_string(),
            wallet,
        })
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self.wallet.get_balance(0, None).await?;
        Ok(balance.unlocked_balance.as_pico())
    }
}
