use axum::{routing::get, Router};
use git_turbine::api::{IndexTemplate, PaidCommit, RepoUrl};

fn create_mock_commit(name: &str, amount: &str, timestamp: u64, commit_id: &str, message: &str) -> PaidCommit {
    PaidCommit {
        contributor_name: name.to_string(),
        amount: amount.to_string(),
        timestamp,
        commit_id: commit_id.to_string(),
        commit_message: message.to_string(),
        currency: "XMR".to_string(),
    }
}

async fn index() -> IndexTemplate {
    IndexTemplate {
        monero_enabled: true,
        monero_balance: "12.34567".to_string(),
        monero_balance_usd: "1500.25".to_string(),
        monero_block_height: 2_500_000,
        monero_network: "Main".to_string(),
        monero_wallet_address: "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge".to_string(),
        repository_url: RepoUrl::new("https://github.com/fossable/turbine".to_string()),
        commits: vec![
            create_mock_commit("Alice", "0.50000", 1234567890, "a1b2c3d", "fix: resolve payment calculation bug"),
            create_mock_commit("Bob", "1.25000", 1234567900, "e4f5g6h", "feat: add new cryptocurrency support"),
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

    println!("Serving UI with Monero enabled at http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
