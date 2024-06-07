use anyhow::Result;
use axum::{
    extract::FromRef,
    routing::{get, post},
    Router,
};
use clap::{Args, Parser};
use std::{env, process::ExitCode, sync::Arc};
use tokio::spawn;
use tokio::{net::TcpListener, sync::Mutex};
use tracing::info;

use crate::repo::TurbineRepo;

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

    #[cfg(feature = "monero")]
    #[clap(long, default_value = "1234")]
    pub monero_wallet_password: String,

    #[cfg(feature = "monero")]
    #[clap(long)]
    pub monero_wallet: String,

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

    info!("Starting listener");
    let listener =
        TcpListener::bind(args.bind.as_ref().unwrap_or(&"0.0.0.0:3000".to_string())).await?;
    axum::serve(listener, app.with_state(state)).await?;
    Ok(ExitCode::SUCCESS)
}
