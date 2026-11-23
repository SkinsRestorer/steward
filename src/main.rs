mod ai;
mod bots;
mod commands;
mod config;
mod download;
mod events;
mod ocr;
mod patterns;
#[cfg(test)]
mod regression_tests;
mod releases;
mod runtime;
mod state;

use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use futures::future::join_all;
use state::SharedServices;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Load the process environment before Tokio starts its worker threads.
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

    run()
}

#[tokio::main]
async fn run() -> Result<()> {
    // Check tokens before loading the OCR models and other shared services.
    let tokens = bots::ALL
        .iter()
        .map(|bot| runtime::bot_token(bot))
        .collect::<Result<Vec<_>>>()?;
    let services = Arc::new(
        SharedServices::load()
            .await
            .context("failed to initialize shared services")?,
    );

    // /latest and the outdated-version check can name different URLs.
    for bot in bots::ALL {
        let latest_url = bot.commands.latest.map(|latest| latest.release_url);
        for url in bot.checks.release_url.into_iter().chain(latest_url) {
            services.releases.start_refreshing(url);
        }
    }

    let (shutdown_sender, shutdown) = watch::channel(false);
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        tracing::info!("shutting down; send the signal again to exit immediately");
        shutdown_sender.send_replace(true);
        // Tokio keeps handling the signals, so without this a stuck shutdown
        // could only be ended with SIGKILL.
        wait_for_shutdown_signal().await;
        tracing::warn!("received a second shutdown signal; exiting without cleanup");
        #[expect(
            clippy::exit,
            reason = "A second signal must end the process even when shutdown is stuck."
        )]
        std::process::exit(1);
    });

    let results =
        join_all(bots::ALL.iter().copied().zip(tokens).map(|(bot, token)| {
            runtime::run_bot(bot, token, Arc::clone(&services), shutdown.clone())
        }))
        .await;
    services.logging.shutdown().await;

    results.into_iter().collect()
}

/// Waits for Ctrl+C or, on Unix, SIGTERM from a container stop or deploy.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = terminate.recv() => {}
                }
                return;
            }
            Err(error) => {
                tracing::warn!(
                    error = %format_args!("{error:#}"),
                    "failed to listen for SIGTERM"
                );
            }
        }
    }
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!(error = %format_args!("{error:#}"), "failed to listen for Ctrl+C");
        std::future::pending::<()>().await;
    }
}
