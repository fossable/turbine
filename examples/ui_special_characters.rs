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
        monero_balance: "5.00000".to_string(),
        monero_balance_usd: "750.00".to_string(),
        monero_block_height: 2_000_000,
        monero_network: "Main".to_string(),
        monero_wallet_address: "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge".to_string(),
        repository_url: "https://github.com/test/repo".to_string(),
        commits: vec![
            create_mock_commit("John O'Brien", "0.50000", 1234567890),
            create_mock_commit("José García", "0.75000", 1234567900),
            create_mock_commit("<script>alert('xss')</script>", "0.25000", 1234567910),
        ],
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
