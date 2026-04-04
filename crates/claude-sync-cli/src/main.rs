mod commands;

use clap::Parser;
use commands::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    commands::execute(cli).await
}
