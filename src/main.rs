mod ai;
mod bots;
mod commands;
mod config;
mod events;
mod ocr;
mod releases;
mod runtime;
mod state;

use std::sync::Arc;

use anyhow::{Context as _, Result};
use futures::future::try_join_all;
use state::SharedServices;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("steward=info,warn")),
        )
        .init();

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
