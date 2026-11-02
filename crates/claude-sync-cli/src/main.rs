mod commands;

use clap::Parser;
use commands::Cli;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .init();

    commands::execute(Cli::parse())
}
