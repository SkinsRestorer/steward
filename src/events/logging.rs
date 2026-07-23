use std::path::Path;

use anyhow::{Context as _, Result};
use chrono::Utc;
use chrono_tz::Europe::Berlin;
use poise::serenity_prelude as serenity;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt as _,
};

use crate::state::AppState;

pub async fn handle(
    data: &AppState,
    message: &serenity::Message,
    channel_name: Option<&str>,
) -> Result<()> {
    let Some(channel_name) = channel_name else {
        return Ok(());
    };

    let now = Utc::now().with_timezone(&Berlin);
    fs::create_dir_all(data.bot.logs_dir)
        .await
        .with_context(|| format!("failed to create {}", data.bot.logs_dir))?;
    let path = Path::new(data.bot.logs_dir).join(format!("{}.log", now.format("%Y-%m-%d")));
    let line = format!(
        "{} [{}] {}: {}\n",
        now.format("%I-%M-%S %p"),
        channel_name,
        message.author.tag(),
        message.content
    );
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .with_context(|| format!("failed to open {}", path.display()))?;
    file.write_all(line.as_bytes())
        .await
        .with_context(|| format!("failed to append to {}", path.display()))?;
    Ok(())
}
