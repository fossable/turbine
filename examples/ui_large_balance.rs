use askama_axum::Template;
use axum::{routing::get, Router};
use git_turbine::api::{IndexTemplate, PaidCommit};

fn create_mock_commit(name: &str, amount: &str, timestamp: u64) -> PaidCommit {
    PaidCommit {
        contributor_name: name.to_string(),
        amount: amount.to_string(),
        timestamp,
    }
}

async fn index() -> IndexTemplate {
    IndexTemplate {
        monero_enabled: true,
        monero_balance: "1234567.89012".to_string(),
        monero_balance_usd: "185000000.00".to_string(),
        monero_block_height: 4_000_000,
        monero_network: "Main".to_string(),
        monero_wallet_address: "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge".to_string(),
        repository_url: "https://github.com/whale/project".to_string(),
        commits: vec![create_mock_commit("Whale", "1000000.00000", 1234567890)],
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
