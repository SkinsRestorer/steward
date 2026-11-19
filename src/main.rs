mod ai;
mod bots;
mod commands;
mod config;
mod download;
mod events;
mod ocr;
mod patterns;
mod releases;
mod runtime;
mod state;

use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use futures::future::try_join_all;
use state::SharedServices;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let dotenv_result = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("steward=info,warn")),
        )
        .try_init()
        .map_err(|error| anyhow!("failed to initialize tracing: {error}"))?;

    match dotenv_result {
        Ok(_) => {}
        Err(error) if error.not_found() => {}
        Err(dotenvy::Error::Io(error)) => {
            tracing::warn!(kind = ?error.kind(), "failed to read .env");
        }
        Err(dotenvy::Error::LineParse(_, index)) => {
            tracing::warn!(index, "failed to parse .env");
        }
        Err(_) => tracing::warn!("failed to load .env"),
    }

    let services = Arc::new(
        SharedServices::load()
            .await
            .context("failed to initialize shared services")?,
    );

    for bot in bots::ALL {
        if let Some(url) = bot.checks.release_url {
            services.releases.start_refreshing(url);
        }
    }

    try_join_all(
        bots::ALL
            .iter()
            .copied()
            .map(|bot| runtime::start_bot(bot, Arc::clone(&services))),
    )
    .await?;

    Ok(())
}
