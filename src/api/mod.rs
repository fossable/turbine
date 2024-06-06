use crate::cli::AppState;
use askama_axum::Template;
use axum::extract::State;
use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use cached::proc_macro::once;
use rust_embed::Embed;
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
pub async fn refresh(State(state): State<AppState>) {
    let mut repo = state.repo.lock().await;
}
