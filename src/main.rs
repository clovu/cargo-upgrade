mod app;
mod cli;
mod dependency;
mod error;
mod grouped_checklist;
mod manifest;
mod registry;
mod versioning;

use clap::Parser;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    app::run(cli::Cli::parse()).await
}
