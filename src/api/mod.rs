use crate::cli::AppState;
use askama_axum::Template;
use axum::extract::State;
use cached::proc_macro::once;
use tracing::instrument;

#[derive(Template, Debug, Clone)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    #[cfg(feature = "monero")]
    monero_balance: String,
    #[cfg(feature = "monero")]
    monero_wallet_address: String,
}

#[once(time = "60")]
#[instrument(ret)]
pub async fn index(State(state): State<AppState>) -> IndexTemplate {
    IndexTemplate {
        #[cfg(feature = "monero")]
        monero_balance: format!("{}", state.monero.get_balance().await.unwrap()),
        #[cfg(feature = "monero")]
        monero_wallet_address: state.monero.wallet_address.clone(),
    }
}
