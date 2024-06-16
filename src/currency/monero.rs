use crate::cli::{AppState, ServeArgs};
use anyhow::Result;
use axum::extract::State;
use axum::response::IntoResponse;

use float_pretty_print::PrettyPrintFloat;
use git2::Oid;
use monero_rpc::{
    monero::{Address, Amount},
    BlockHeightFilter, GetTransfersCategory, GetTransfersSelector, GotTransfer,
    RestoreDeterministicWalletArgs, RpcClientBuilder, TransferOptions, TransferPriority,
    WalletClient,
};
use reqwest::header;
use std::{
    collections::HashMap,
    process::{Child, Command},
    sync::Arc,
    time::Duration,
};
use std::{str::FromStr, sync::Mutex};
use tracing::{debug, info, instrument};

#[derive(Clone, Debug)]
pub struct MoneroState {
    pub wallet: WalletClient,
    wallet_process: Option<Arc<Mutex<Child>>>,
    pub wallet_address: Address,
    account_index: u32,
    minimum_block_height: u64,
}

impl Drop for MoneroState {
    fn drop(&mut self) {
        if let Some(process) = self.wallet_process.as_mut() {
            if let Some(process) = Arc::get_mut(process) {
                let mut process = process.lock().unwrap();

                debug!("Stopping RPC wallet daemon");
                process.kill().unwrap_or_default();
            }
        }
    }
}

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
            .arg("--wallet-dir")
            .arg("/wallets")
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

        if args.monero_wallet_seed {
            debug!("Restoring wallet from mnemonic seed phrase");
            wallet
                .restore_deterministic_wallet(RestoreDeterministicWalletArgs {
                    autosave_current: None,
                    filename: "turbine".into(),
                    password: args.monero_wallet_password.clone(),
                    restore_height: Some(args.monero_block_height),
                    seed: std::env::var("MONERO_WALLET_SEED")?,
                    seed_offset: None,
                })
                .await?;
        } else if let Some(path) = args.monero_wallet_path.as_ref() {
            wallet
                .open_wallet(path.to_owned(), Some(args.monero_wallet_password.clone()))
                .await?;
        }

        info!(
            block_height = wallet.get_height().await?,
            "Connected to wallet RPC"
        );

        Ok(Self {
            wallet_process: Some(Arc::new(Mutex::new(wallet_process))),
            wallet_address: wallet.get_address(0, None).await?.address,
            wallet,
            account_index: 0,
            minimum_block_height: args.monero_block_height,
        })
    }

    /// Query the current wallet balance.
    // #[once(time = "60")]
    pub async fn get_balance(&self) -> Result<Amount> {
        let balance = self.wallet.get_balance(self.account_index, None).await?;
        debug!(balance = ?balance, "Current Monero wallet balance");
        Ok(balance.balance)
    }

    /// Count outbound transfers to the given address.
    pub async fn count_transfers(&self, address: &str) -> Result<usize> {
        let transfers = self
            .wallet
            .get_transfers(GetTransfersSelector {
                category_selector: HashMap::from([(GetTransfersCategory::Out, true)]),
                account_index: Some(self.account_index),
                subaddr_indices: None,
                block_height_filter: Some(BlockHeightFilter {
                    min_height: Some(self.minimum_block_height),
                    max_height: None,
                }),
            })
            .await?;

        Ok(transfers
            .get(&GetTransfersCategory::Out)
            .unwrap_or(&vec![])
            .iter()
            .filter(|transfer| transfer.address.to_string() == address.to_string())
            .count())
    }

    /// Get all outbound transfers.
    #[instrument(skip_all, ret)]
    pub async fn get_transfers(&self) -> Result<Vec<GotTransfer>> {
        let transfers = self
            .wallet
            .get_transfers(GetTransfersSelector {
                category_selector: HashMap::from([(GetTransfersCategory::Out, true)]),
                account_index: Some(self.account_index),
                subaddr_indices: None,
                block_height_filter: Some(BlockHeightFilter {
                    min_height: Some(self.minimum_block_height),
                    max_height: None,
                }),
            })
            .await?;

        Ok(transfers
            .get(&GetTransfersCategory::Out)
            .unwrap_or(&vec![])
            .to_owned())
    }

    /// Transfer the given amount of Monero.
    pub async fn transfer(&self, address: &str, amount: Amount, _commit_id: &Oid) -> Result<()> {
        info!(amount = ?amount, dest = ?address, "Transferring Monero");
        self.wallet
            .transfer(
                HashMap::from([(Address::from_str(address)?, amount)]),
                TransferPriority::Default,
                TransferOptions {
                    account_index: Some(self.account_index),
                    subaddr_indices: None,
                    mixin: None,
                    ring_size: Some(16),
                    unlock_time: None,
                    payment_id: None,
                    do_not_relay: None,
                },
            )
            .await?;
        Ok(())
    }
}

/// Return an SVG badge with the current monero balance.
pub async fn balance(State(state): State<AppState>) -> impl IntoResponse {
    let monero_balance = state.monero.get_balance().await.unwrap().as_xmr();

    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        crate::badge::generate(
            "balance",
            &format!("{} XMR", PrettyPrintFloat(monero_balance)),
        ),
    )
}

/// Return an SVG badge with the total number of payouts.
pub async fn payouts(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        crate::badge::generate(
            "payouts",
            &format!("{}", state.monero.get_transfers().await.unwrap().len()),
        ),
    )
}
