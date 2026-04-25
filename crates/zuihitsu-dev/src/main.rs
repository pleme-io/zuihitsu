//! zuihitsu-dev — entry point and command dispatch.
//!
//! One binary, four subcommands. The daemon is the interesting one; the
//! others are short-lived CLI helpers that the dev loop occasionally needs.

use clap::{Parser, Subcommand};

mod cmd;
mod daemon;

#[derive(Parser)]
#[command(
    name = "zuihitsu-dev",
    version,
    about = "zuihitsu — dev daemon + authoring CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Watch sources, rebuild on change, serve dist/ with hot reload.
    Daemon(daemon::Args),
    /// Invalidate the Hashnode response cache and re-warm via one sitegen pass.
    Fetch(cmd::fetch::Args),
    /// Scaffold a new markdown draft under drafts/<slug>.md.
    Draft(cmd::draft::Args),
    /// POST a signed mock Hashnode webhook to a local zuihitsu-worker.
    WorkerTest(cmd::worker_test::Args),
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    match Cli::parse().command {
        Command::Daemon(a) => daemon::run(a).await,
        Command::Fetch(a) => cmd::fetch::run(a).await,
        Command::Draft(a) => cmd::draft::run(a).await,
        Command::WorkerTest(a) => cmd::worker_test::run(a).await,
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,zuihitsu_dev=debug".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
