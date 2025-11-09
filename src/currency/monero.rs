use crate::cli::{AppState, ServeArgs};
use anyhow::Result;
use axum::extract::State;
use axum::response::IntoResponse;

use float_pretty_print::PrettyPrintFloat;
use git2::Oid;
use monero_rpc::{
    monero::{Address, Amount, PrivateKey},
    BlockHeightFilter, GenerateFromKeysArgs, GetTransfersCategory, GetTransfersSelector,
    GotTransfer, RestoreDeterministicWalletArgs, RpcClientBuilder, TransferOptions, TransferPriority, WalletClient,
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

/// Derive a deterministic subaddress index from a commit OID.
/// This ensures each commit maps to a unique subaddress for idempotent payments.
pub fn commit_to_subaddress_index(commit_id: Oid) -> u32 {
    let hash = commit_id.as_bytes();
    // Use first 4 bytes of commit hash as subaddress index
    u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
}

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

        let wallet_dir = args.monero_wallet_path.as_ref()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("/wallets"));

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
            .arg(wallet_dir)
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

        if let Some(path) = args.monero_wallet_path.as_ref() {
            let wallet_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path.to_str().unwrap_or("wallet"));
            wallet
                .open_wallet(wallet_name.to_owned(), Some(args.monero_wallet_password.clone()))
                .await?;
        } else if let Ok(seed) = std::env::var("MONERO_WALLET_SEED") {
            debug!("Restoring wallet from mnemonic seed phrase");
            wallet
                .restore_deterministic_wallet(RestoreDeterministicWalletArgs {
                    autosave_current: None,
                    filename: "turbine".into(),
                    password: args.monero_wallet_password.clone(),
                    restore_height: Some(args.monero_block_height),
                    seed,
                    seed_offset: None,
                })
                .await?;
        } else {
            wallet
                .generate_from_keys(GenerateFromKeysArgs {
                    restore_height: Some(args.monero_block_height),
                    filename: "turbine".into(),
                    address: Address::from_str(&std::env::var("MONERO_WALLET_ADDRESS")?)?,
                    spendkey: Some(PrivateKey::from_str(&std::env::var(
                        "MONERO_WALLET_SPENDKEY",
                    )?)?),
                    viewkey: PrivateKey::from_str(&std::env::var("MONERO_WALLET_VIEWKEY")?)?,
                    password: args.monero_wallet_password.clone(),
                    autosave_current: None,
                })
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

    /// Check if a specific commit has already been paid by checking its dedicated subaddress.
    /// This is stateless and idempotent - the commit OID deterministically maps to a subaddress.
    #[instrument(skip(self), ret)]
    pub async fn is_commit_paid(&self, commit_id: Oid) -> Result<bool> {
        let subaddress_index = commit_to_subaddress_index(commit_id);

        let transfers = self
            .wallet
            .get_transfers(GetTransfersSelector {
                category_selector: HashMap::from([(GetTransfersCategory::Out, true)]),
                account_index: Some(self.account_index),
                subaddr_indices: Some(vec![subaddress_index]),
                block_height_filter: Some(BlockHeightFilter {
                    min_height: Some(self.minimum_block_height),
                    max_height: None,
                }),
            })
            .await?;

        Ok(!transfers
            .get(&GetTransfersCategory::Out)
            .unwrap_or(&vec![])
            .is_empty())
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

    /// Transfer the given amount of Monero from a commit-specific subaddress.
    /// Each commit gets a unique subaddress derived from its OID, ensuring idempotent payments.
    pub async fn transfer(&self, address: &str, amount: Amount, commit_id: &Oid) -> Result<()> {
        let subaddress_index = commit_to_subaddress_index(*commit_id);

        info!(
            amount = ?amount,
            dest = ?address,
            commit = ?commit_id,
            subaddr_index = subaddress_index,
            "Transferring Monero from commit-specific subaddress"
        );

        self.wallet
            .transfer(
                HashMap::from([(Address::from_str(address)?, amount)]),
                TransferPriority::Default,
                TransferOptions {
                    account_index: Some(self.account_index),
                    subaddr_indices: Some(vec![subaddress_index]),
                    mixin: None,
                    ring_size: Some(16),
                    unlock_time: None,
                    payment_id: None,
                    do_not_relay: None,
                    subtract_fee_from_outputs: None,
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
            &format!("{:.1} XMR", PrettyPrintFloat(monero_balance)),
        )
        .await
        .unwrap(),
    )
}

/// Return an SVG badge with the total number of payouts.
pub async fn payouts(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        crate::badge::generate(
            "payouts",
            &format!("{}", state.monero.get_transfers().await.unwrap().len()),
        )
        .await
        .unwrap(),
    )
}

/// Return an SVG badge with the wallet address.
pub async fn address(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        crate::badge::generate(
            "XMR",
            &format!("{}", state.monero.wallet_address.to_string()),
        )
        .await
        .unwrap(),
    )
}
