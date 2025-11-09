use askama_axum::Template;
use axum::{routing::get, Router};
use git_turbine::api::IndexTemplate;

async fn index() -> IndexTemplate {
    IndexTemplate {
        monero_enabled: false,
        monero_balance: String::new(),
        monero_balance_usd: String::new(),
        monero_block_height: 0,
        monero_network: String::new(),
        monero_wallet_address: String::new(),
        repository_url: "https://github.com/fossable/turbine".to_string(),
        commits: vec![],
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/assets/*file", get(git_turbine::api::assets));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("Failed to bind to port 8080");

    axum::serve(listener, app).await.unwrap();
}
