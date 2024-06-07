use crate::{cli::ServeArgs, CommandLine};
use anyhow::Result;
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    Json,
};
use cached::proc_macro::once;
use monero_rpc::{
    monero::{Address, Amount},
    GetTransfersCategory, GetTransfersSelector, RpcClientBuilder, TransferOptions,
    TransferPriority, WalletClient,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{
    collections::HashMap,
    process::{Child, Command},
    sync::Arc,
    time::Duration,
};
use tracing::{debug, info, instrument};

#[derive(Clone, Debug)]
pub struct MoneroState {
    wallet: WalletClient,
    wallet_process: Option<Arc<Child>>,
    pub wallet_address: String,
    account_index: u32,
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
            account_index: 0,
        })
    }

    #[instrument(ret)]
    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self.wallet.get_balance(self.account_index, None).await?;
        Ok(balance.unlocked_balance.as_pico())
    }

    /// Count outbound transfers to the given address.
    #[instrument(ret)]
    pub async fn count_transfers(&self, address: &str) -> Result<usize> {
        let transfers = self
            .wallet
            .get_transfers(GetTransfersSelector {
                category_selector: HashMap::from([(GetTransfersCategory::Out, true)]),
                account_index: Some(self.account_index),
                subaddr_indices: Some(vec![
                    self.wallet
                        .get_address_index(Address::from_str(address)?)
                        .await?
                        .minor,
                ]),
                block_height_filter: None,
            })
            .await?;

        Ok(transfers
            .get(&GetTransfersCategory::Out)
            .unwrap()
            .iter()
            .filter(|transfer| transfer.address.to_string() == address.to_string())
            .count())
    }

    #[instrument(ret)]
    pub async fn transfer(&self, address: &str, amount: Amount) -> Result<()> {
        self.wallet
            .transfer(
                HashMap::from([(Address::from_str(address)?, amount)]),
                TransferPriority::Default,
                TransferOptions {
                    account_index: Some(self.account_index),
                    subaddr_indices: None,
                    mixin: None,
                    ring_size: None,
                    unlock_time: None,
                    payment_id: None,
                    do_not_relay: None,
                },
            )
            .await?;
        Ok(())
    }
}
