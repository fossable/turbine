use anyhow::Result;
use axum::{
    extract::FromRef,
    routing::{get, post},
    Router,
};
use clap::{Args, Parser};
use std::{env, process::ExitCode, sync::Arc};
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::info;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    Serve(ServeArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ServeArgs {
    bind: Option<String>,
    // #[cfg(feature = "monero")]
    // monero_wallet_url: String,
    #[cfg(feature = "monero")]
    #[clap(long, num_args = 0)]
    stagenet: bool,
    #[cfg(feature = "monero")]
    #[clap(long, num_args = 0)]
    testnet: bool,
}

#[derive(Clone, Debug)]
pub struct AppState {
    #[cfg(feature = "monero")]
    pub monero: crate::currency::monero::MoneroState,
}

pub async fn serve(args: &ServeArgs) -> Result<ExitCode> {
    let state = AppState {
        #[cfg(feature = "monero")]
        monero: crate::currency::monero::MoneroState::new(&args).await?,
    };

    let app = Router::new().route("/", get(crate::api::index));

    #[cfg(feature = "monero")]
    let app = app.route("/xmr/provision", post(crate::currency::monero::provision));

    info!("Starting listener");
    let listener =
        TcpListener::bind(args.bind.as_ref().unwrap_or(&"0.0.0.0:3000".to_string())).await?;
    axum::serve(listener, app.with_state(state)).await?;
    Ok(ExitCode::SUCCESS)
}
