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
    let commits: Vec<PaidCommit> = (0..20)
        .map(|i| {
            create_mock_commit(
                &format!("Contributor_{}", i),
                &format!("{:.5}", (i as f64) * 0.1),
                1234567890 + (i * 10),
            )
        })
        .collect();

    IndexTemplate {
        monero_enabled: true,
        monero_balance: "50.00000".to_string(),
        monero_balance_usd: "7500.00".to_string(),
        monero_block_height: 3_000_000,
        monero_network: "Main".to_string(),
        monero_wallet_address: "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge".to_string(),
        repository_url: "https://github.com/popular/repo".to_string(),
        commits,
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
