use crate::cli::AppState;
use askama_axum::Template;
use axum::extract::State;
use cached::proc_macro::once;
use tracing::instrument;

#[derive(Template, Debug)]
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
        monero_balance: state.monero_balance(),
        #[cfg(feature = "monero")]
        monero_wallet_address: state.monero_wallet_address.clone(),
    }
}
