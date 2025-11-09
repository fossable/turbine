use axum::{routing::get, Router};
use git_turbine::api::{IndexTemplate, RepoUrl};

async fn index() -> IndexTemplate {
    IndexTemplate {
        monero_enabled: true,
        monero_balance: "0.00000".to_string(),
        monero_balance_usd: "0.00".to_string(),
        monero_block_height: 2_800_000,
        monero_network: "Main".to_string(),
        monero_wallet_address: "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge".to_string(),
        repository_url: RepoUrl::new("https://github.com/fossable/turbine".to_string()),
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
