mod args;
mod commands;
mod config;
mod output;

use args::{Cli, Commands};
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cli.verbose)),
        )
        .init();

    // Load TOML config (non-existent file uses defaults)
    let _app_config = config::load_config(&cli.config)?;

    match cli.command {
        Commands::Capture {
            listen,
            include_domains,
            exclude_domains,
            storage,
            output_dir,
        } => {
            commands::capture::execute(
                &listen,
                &include_domains,
                &exclude_domains,
                &storage,
                &output_dir,
            )
            .await?;
        }
        Commands::Cert { action } => {
            commands::cert::execute(action).await?;
        }
        Commands::Replay { input, target } => {
            commands::replay::execute(&input, target.as_deref()).await?;
        }
    }

    Ok(())
}
