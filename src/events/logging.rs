use std::{collections::HashMap, path::Path};

use anyhow::{Context as _, Result};
use chrono::Utc;
use chrono_tz::Europe::Berlin;
use poise::serenity_prelude as serenity;
use serde::Serialize;
use tokio::{
    fs::{self, File, OpenOptions},
    io::AsyncWriteExt as _,
    sync::mpsc,
};

use crate::{config::BotDefinition, state::AppState};

const LOG_QUEUE_CAPACITY: usize = 512;

#[derive(Clone)]
pub struct LoggingService {
    senders: HashMap<&'static str, mpsc::Sender<LogRecord>>,
}

#[derive(Serialize)]
struct LogRecord {
    timestamp: String,
    channel: String,
    author: String,
    content: String,
    #[serde(skip)]
    date: String,
}

impl LoggingService {
    pub async fn start(bots: &[&'static BotDefinition]) -> Result<Self> {
        let mut senders = HashMap::new();

        for bot in bots {
            fs::create_dir_all(bot.logs_dir)
                .await
                .with_context(|| format!("failed to create {}", bot.logs_dir))?;
            let (sender, receiver) = mpsc::channel(LOG_QUEUE_CAPACITY);
            tokio::spawn(run_writer(bot.logs_dir, receiver));
            senders.insert(bot.id, sender);
        }

        Ok(Self { senders })
    }

    fn write(
        &self,
        bot: &BotDefinition,
        message: &serenity::Message,
        channel_name: &str,
    ) -> Result<()> {
        let now = Utc::now().with_timezone(&Berlin);
        let record = LogRecord {
            timestamp: now.to_rfc3339(),
            channel: channel_name.to_owned(),
            author: message.author.tag(),
            content: message.content.clone(),
            date: now.format("%Y-%m-%d").to_string(),
        };
        let sender = self
            .senders
            .get(bot.id)
            .with_context(|| format!("no log writer configured for {}", bot.name))?;
        sender
            .try_send(record)
            .with_context(|| format!("{} log queue is unavailable", bot.name))
    }
}

pub fn handle(
    data: &AppState,
    message: &serenity::Message,
    channel_name: Option<&str>,
) -> Result<()> {
    let Some(channel_name) = channel_name else {
        return Ok(());
    };

    data.services.logging.write(data.bot, message, channel_name)
}

async fn run_writer(logs_dir: &'static str, mut receiver: mpsc::Receiver<LogRecord>) {
    let mut current_date = String::new();
    let mut file = None;

    while let Some(record) = receiver.recv().await {
        if current_date != record.date {
            match open_log_file(logs_dir, &record.date).await {
                Ok(next_file) => {
                    current_date.clone_from(&record.date);
                    file = Some(next_file);
                }
                Err(error) => {
                    tracing::error!(%error, %logs_dir, "failed to rotate message log");
                    file = None;
                    continue;
                }
            }
        }

        let Some(active_file) = file.as_mut() else {
            continue;
        };
        let mut line = match serde_json::to_vec(&record) {
            Ok(line) => line,
            Err(error) => {
                tracing::error!(%error, %logs_dir, "failed to encode message log record");
                continue;
            }
        };
        line.push(b'\n');
        if let Err(error) = active_file.write_all(&line).await {
            tracing::error!(%error, %logs_dir, "failed to append message log record");
            file = None;
            current_date.clear();
        }
    }
}

async fn open_log_file(logs_dir: &str, date: &str) -> Result<File> {
    let path = Path::new(logs_dir).join(format!("{date}.jsonl"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .with_context(|| format!("failed to open {}", path.display()))
}
