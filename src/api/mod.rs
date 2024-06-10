use crate::cli::AppState;
use askama_axum::Template;
use axum::extract::State;
use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use cached::proc_macro::once;
use chrono::{DateTime, Utc};
use rust_embed::Embed;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub amount: String,
    pub timestamp: u64,
}

#[derive(Template, Debug, Clone, Default)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    monero_balance: String,
    monero_wallet_address: String,
    monero_network: String,
    monero_block_height: u64,
    transactions: Vec<Transaction>,
}

#[once(time = "60")]
pub async fn index(State(state): State<AppState>) -> IndexTemplate {
    IndexTemplate {
        #[cfg(feature = "monero")]
        monero_balance: format!(
            "{:.2e}",
            state.monero.get_balance().await.unwrap() as f64 / f64::powf(10.0, 12.0)
        ),
        #[cfg(feature = "monero")]
        monero_wallet_address: state.monero.wallet_address.to_string(),
        #[cfg(feature = "monero")]
        monero_network: match state.monero.wallet_address.network {
            monero_rpc::monero::Network::Mainnet => "Main",
            monero_rpc::monero::Network::Stagenet => "Stage",
            monero_rpc::monero::Network::Testnet => "Test",
        }
        .to_string(),
        #[cfg(feature = "monero")]
        monero_block_height: state.monero.wallet.get_height().await.unwrap().get(),
        transactions: vec![Transaction {
            amount: "123".into(),
            timestamp: 123,
        }],
        ..Default::default()
    }
}

// We use a wildcard matcher ("/dist/*file") to match against everything
// within our defined assets directory. This is the directory on our Asset
// struct below, where folder = "examples/public/".
pub async fn assets(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("assets/") {
        path = path.replace("assets/", "");
    }

    StaticFile(path)
}

#[derive(Embed)]
#[folder = "assets/"]
struct Asset;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

/// Refresh the turbine repo
#[once(time = "60")]
#[instrument(ret)]
pub async fn refresh(State(state): State<AppState>) {
    let mut repo = state.repo.lock().await;
    repo.refresh().unwrap();

    for contributor in repo.contributors.iter() {
        match contributor.address.clone() {
            crate::currency::Address::BTC(_) => todo!(),
            #[cfg(feature = "monero")]
            crate::currency::Address::XMR(address) => {
                let transfer_count = state.monero.count_transfers(&address).await.unwrap();
                for commit_id in contributor.commits.iter().skip(transfer_count) {
                    state
                        .monero
                        .transfer(
                            &address,
                            monero::Amount::from_pico(
                                contributor.compute_payout(commit_id.clone()),
                            ),
                        )
                        .await
                        .unwrap();
                }
            }
        };
    }
}
