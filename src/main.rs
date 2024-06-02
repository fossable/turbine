use crate::cli::Commands;
use anyhow::Result;
use axum::{response::Html, routing::get, Router};
use clap::Parser;
use std::process::ExitCode;

mod api;
mod cli;
mod config;
mod currency;
mod repo;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandLine {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let args = CommandLine::parse();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Dispatch command
    match &args.command {
        Some(Commands::Serve(args)) => crate::cli::serve(args).await,
        None => todo!(),
    }
}
