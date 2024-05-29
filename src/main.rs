use crate::cli::Commands;
use axum::{response::Html, routing::get, Router};
use clap::Parser;

mod api;
mod cli;
mod config;
mod currency;
mod repo;

// async fn main() {
//     // build our application with a route
//     let app = Router::new().route("/", get(todo!()));

//     // run it
//     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//         .await
//         .unwrap();
//     axum::serve(listener, app).await.unwrap();
// }

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandLine {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() {
    let command_line = CommandLine::parse();

    // Configure logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Dispatch command
    match &command_line.command {
        Some(Commands::Provision {}) => todo!(),
        Some(Commands::Deprovision {}) => todo!(),
        None => todo!(),
    }
}
