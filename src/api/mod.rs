use crate::cli::AppState;
use askama_axum::Template;
use axum::extract::State;
use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use cached::proc_macro::once;
use rust_embed::Embed;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct RepoUrl(String);

impl RepoUrl {
    pub fn new(url: String) -> Self {
        Self(url)
    }

    pub fn is_github(&self) -> bool {
        self.0.starts_with("https://github.com")
    }
}

impl std::fmt::Display for RepoUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct PaidCommit {
    pub amount: String,
    pub timestamp: u64,
    pub contributor_name: String,
    pub commit_id: String,
    pub commit_message: String,
    pub currency: String,
}

#[derive(Template, Debug, Clone, Default)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub monero_balance: String,
    pub monero_enabled: bool,
    pub monero_block_height: u64,
    pub monero_network: String,
    pub monero_wallet_address: String,
    pub repository_url: RepoUrl,
    pub commits: Vec<PaidCommit>,
    pub monero_balance_usd: String,
}

#[once(time = "60")]
#[debug_handler]
pub async fn index(State(state): State<AppState>) -> IndexTemplate {
    #[cfg(feature = "monero")]
    let monero_balance = state.monero.get_balance().await.unwrap().as_xmr();
    let repo = state.repo.lock().await;

    IndexTemplate {
        monero_enabled: cfg!(feature = "monero"),
        #[cfg(feature = "monero")]
        monero_balance: format!("{:.5}", monero_balance),
        #[cfg(feature = "monero")]
        monero_block_height: state.monero.wallet.get_height().await.unwrap().get(),
        #[cfg(feature = "monero")]
        monero_network: match state.monero.wallet_address.network {
            monero_rpc::monero::Network::Mainnet => "Main",
            monero_rpc::monero::Network::Stagenet => "Stage",
            monero_rpc::monero::Network::Testnet => "Test",
        }
        .to_string(),
        #[cfg(feature = "monero")]
        monero_wallet_address: state.monero.wallet_address.to_string(),
        repository_url: repo.remote.clone(),
        #[cfg(feature = "monero")]
        commits: state
            .monero
            .get_transfers()
            .await
            .unwrap()
            .iter()
            .filter_map(|transfer| repo.find_monero_transaction(transfer).ok())
            .collect(),
        #[cfg(feature = "monero")]
        monero_balance_usd: format!(
            "{:.2}",
            crate::currency::lookup("XMR").await.unwrap_or(0.0) * monero_balance
        ),
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
pub async fn refresh(State(state): State<AppState>) {
    let mut repo = state.repo.lock().await;
    repo.refresh().unwrap();

    for contributor in repo.contributors.iter() {
        match contributor.address.clone() {
            crate::currency::Address::BTC(_) => todo!(),
            #[cfg(feature = "monero")]
            crate::currency::Address::XMR(address) => {
                debug!(address = ?address, total_commits = contributor.commits.len(), "Processing XMR contributor");

                for commit_id in contributor.commits.iter() {
                    // Check if this commit was already paid using its dedicated subaddress
                    match state.monero.is_commit_paid(*commit_id).await {
                        Ok(true) => {
                            debug!(commit = ?commit_id, "Commit already paid, skipping");
                            continue;
                        }
                        Ok(false) => {
                            // Not paid yet, proceed with transfer
                            let payout = contributor.compute_payout(*commit_id, state.base_payout, state.max_payout_cap);
                            match state
                                .monero
                                .transfer(
                                    &address,
                                    monero_rpc::monero::Amount::from_pico(payout),
                                    commit_id,
                                )
                                .await
                            {
                                Ok(_) => info!(commit = ?commit_id, amount = payout, "Transfer complete"),
                                Err(e) => error!(commit = ?commit_id, error=%e, "Transfer failed"),
                            };
                        }
                        Err(e) => {
                            error!(commit = ?commit_id, error=%e, "Failed to check if commit was paid");
                        }
                    }
                }
            }
        };
    }
}
