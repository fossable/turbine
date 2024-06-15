use crate::repo::TurbineRepo;
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use chrono::Utc;
use clap::Args;
use std::{process::ExitCode, sync::Arc};

use tokio::{net::TcpListener, sync::Mutex};
use tokio_schedule::{every, Job};
use tracing::info;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    Serve(ServeArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ServeArgs {
    #[clap(long)]
    pub repo: String,
    #[clap(long, default_value = "master")]
    pub branch: String,
    #[clap(long)]
    pub bind: Option<String>,
    // #[cfg(feature = "monero")]
    // monero_wallet_url: String,
    #[cfg(feature = "monero")]
    #[clap(long, num_args = 0)]
    pub stagenet: bool,

    #[cfg(feature = "monero")]
    #[clap(long, num_args = 0)]
    pub testnet: bool,

    #[cfg(feature = "monero")]
    #[clap(long, default_value_t = 9000)]
    pub monero_rpc_port: u16,

    /// Minimum block height
    #[cfg(feature = "monero")]
    #[clap(long, default_value_t = 3167951)]
    pub monero_block_height: u64,

    #[cfg(feature = "monero")]
    #[clap(long)]
    pub monero_wallet_password: String,

    #[cfg(feature = "monero")]
    #[clap(long, conflicts_with = "monero_wallet_seed")]
    pub monero_wallet_path: Option<String>,

    /// Restore wallet from a mnemonic seed phrase given by the environment variable:
    /// MONERO_WALLET_SEED.
    #[cfg(feature = "monero")]
    #[clap(
        long,
        num_args = 0,
        conflicts_with = "monero_wallet_path",
        default_value_t = false
    )]
    pub monero_wallet_seed: bool,

    #[cfg(feature = "monero")]
    #[clap(long, default_value = "stagenet.xmr-tw.org:38081")]
    pub monero_daemon_address: String,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub repo: Arc<Mutex<TurbineRepo>>,

    #[cfg(feature = "monero")]
    pub monero: crate::currency::monero::MoneroState,
}

pub async fn serve(args: &ServeArgs) -> Result<ExitCode> {
    let state = AppState {
        repo: Arc::new(Mutex::new(TurbineRepo::new(&args.repo, &args.branch)?)),

        #[cfg(feature = "monero")]
        monero: crate::currency::monero::MoneroState::new(&args).await?,
    };

    let app = Router::new()
        .route("/", get(crate::api::index))
        .route("/refresh", post(crate::api::refresh))
        .route("/assets/*file", get(crate::api::assets));

    #[cfg(feature = "monero")]
    let app = app.route("/xmr/balance", get(crate::currency::monero::balance));

    // Refresh every hour
    let every_hour = every(1)
        .hour()
        .at(10, 30)
        .in_timezone(&Utc)
        .perform(|| async {
            reqwest::Client::new()
                .post("http://127.0.0.1:3000/refresh")
                .send()
                .await
                .unwrap();
        });
    tokio::spawn(every_hour);

    let address = args.bind.clone().unwrap_or("0.0.0.0:3000".to_string());

    info!(address =?address,"Starting API");
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app.with_state(state)).await?;
    Ok(ExitCode::SUCCESS)
}
