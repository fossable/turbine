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
    #[clap(long)]
    pub monero_wallet_path: Option<PathBuf>,

    #[cfg(feature = "monero")]
    #[clap(long, default_value = "stagenet.xmr-tw.org:38081")]
    pub monero_daemon_address: String,

    /// Base payout amount in piconero for the first commit (default: 1000000000 = 0.001 XMR)
    #[clap(long, default_value_t = 1000000000)]
    pub base_payout: u64,

    /// Maximum payout cap in piconero per commit (optional, no default = unlimited)
    #[clap(long)]
    pub max_payout_cap: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub repo: Arc<Mutex<TurbineRepo>>,
    pub base_payout: u64,
    pub max_payout_cap: Option<u64>,

    #[cfg(feature = "monero")]
    pub monero: crate::currency::monero::MoneroState,
}

pub async fn serve(args: &ServeArgs) -> Result<ExitCode> {
    let state = AppState {
        repo: Arc::new(Mutex::new(TurbineRepo::new(&args.repo, &args.branch)?)),
        base_payout: args.base_payout,
        max_payout_cap: args.max_payout_cap,

        #[cfg(feature = "monero")]
        monero: crate::currency::monero::MoneroState::new(&args).await?,
    };

    let app = Router::new()
        .route("/", get(crate::api::index))
        .route("/refresh", post(crate::api::refresh))
        .route("/assets/*file", get(crate::api::assets));

    #[cfg(feature = "monero")]
    let app = app
        .route("/xmr/balance", get(crate::currency::monero::balance))
        .route("/xmr/payouts", get(crate::currency::monero::payouts))
        .route("/xmr/address", get(crate::currency::monero::address));

    let address = args.bind.clone().unwrap_or("0.0.0.0:80".to_string());

    // Refresh every hour
    let every_hour = every(1)
        .hour()
        .at(10, 30)
        .in_timezone(&Utc)
        .perform(|| async move {
            reqwest::Client::new()
                .post(format!("http://127.0.0.1:{}/refresh", 80)) // TODO
                .send()
                .await
                .unwrap();
        });
    tokio::spawn(every_hour);

    info!(address = ?address,"Starting API");
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app.with_state(state)).await?;
    Ok(ExitCode::SUCCESS)
}
