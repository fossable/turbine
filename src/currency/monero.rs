use crate::{cli::ServeArgs, CommandLine};
use anyhow::Result;
use monero_rpc::{
    monero::{Address, Amount},
    AddressData, BlockHeightFilter, GetTransfersCategory, GetTransfersSelector, GotTransfer,
    RestoreDeterministicWalletArgs, RpcClientBuilder, TransferOptions, TransferPriority,
    WalletClient,
};
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
    pub wallet: WalletClient,
    wallet_process: Option<Arc<Child>>,
    pub wallet_address: Address,
    account_index: u32,
    minimum_block_height: u64,
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
            wallet_process: Some(Arc::new(wallet_process)),
            wallet_address: wallet.get_address(0, None).await?.address,
            wallet,
            account_index: 0,
            minimum_block_height: args.monero_block_height,
        })
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self.wallet.get_balance(self.account_index, None).await?;
        debug!(balance = ?balance, "Current Monero wallet balance");
        Ok(balance.unlocked_balance.as_pico())
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
    pub async fn transfer(&self, address: &str, amount: Amount) -> Result<()> {
        info!(amount = ?amount, dest = ?address, "Transferring Monero");
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
