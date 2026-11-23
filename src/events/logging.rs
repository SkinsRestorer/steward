use std::{collections::HashMap, path::Path};

use anyhow::{Context as _, Result};
use chrono::Utc;
use chrono_tz::Europe::Berlin;
use poise::serenity_prelude as serenity;
use serde::Serialize;
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncWriteExt as _, BufWriter},
    sync::{Mutex, mpsc, watch},
    task::JoinHandle,
};

use crate::{config::BotDefinition, state::AppState};

const LOG_QUEUE_CAPACITY: usize = 512;

pub struct LoggingService {
    senders: HashMap<&'static str, mpsc::Sender<LogRecord>>,
    shutdown: watch::Sender<bool>,
    writers: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Serialize)]
struct LogRecord {
    timestamp: String,
    channel: String,
    author: String,
    content: String,
    message_id: String,
    channel_id: String,
    guild_id: Option<String>,
    user_id: String,
    #[serde(skip)]
    date: String,
}

impl LogRecord {
    fn new(
        message: &serenity::Message,
        channel_name: &str,
        now: chrono::DateTime<chrono_tz::Tz>,
    ) -> Self {
        Self {
            timestamp: now.to_rfc3339(),
            channel: channel_name.to_owned(),
            author: message.author.tag(),
            content: message.content.clone(),
            // Snowflakes are strings so log readers that parse JSON numbers as
            // doubles keep every digit.
            message_id: message.id.to_string(),
            channel_id: message.channel_id.to_string(),
            guild_id: message.guild_id.map(|guild_id| guild_id.to_string()),
            user_id: message.author.id.to_string(),
            date: now.format("%Y-%m-%d").to_string(),
        }
    }
}

impl LoggingService {
    pub async fn start(bots: &[&'static BotDefinition]) -> Result<Self> {
        let mut senders = HashMap::new();
        let mut writers = Vec::new();
        let (shutdown, shutdown_receiver) = watch::channel(false);

        for bot in bots {
            fs::create_dir_all(bot.logs_dir)
                .await
                .with_context(|| format!("failed to create {}", bot.logs_dir))?;
            let (sender, receiver) = mpsc::channel(LOG_QUEUE_CAPACITY);
            writers.push(tokio::spawn(run_writer(
                bot.logs_dir,
                receiver,
                shutdown_receiver.clone(),
            )));
            senders.insert(bot.id, sender);
        }

        Ok(Self {
            senders,
            shutdown,
            writers: Mutex::new(writers),
        })
    }

    /// Writes every queued record, flushes the files, and stops the writers.
    pub async fn shutdown(&self) {
        self.shutdown.send_replace(true);
        let writers = std::mem::take(&mut *self.writers.lock().await);
        for writer in writers {
            if let Err(error) = writer.await {
                tracing::error!(error = %format_args!("{error:#}"), "message log writer failed");
            }
        }
    }

    fn write(
        &self,
        bot: &BotDefinition,
        message: &serenity::Message,
        channel_name: &str,
    ) -> Result<()> {
        let record = LogRecord::new(message, channel_name, Utc::now().with_timezone(&Berlin));
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

async fn run_writer(
    logs_dir: &'static str,
    mut receiver: mpsc::Receiver<LogRecord>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut current_date = String::new();
    let mut file: Option<BufWriter<File>> = None;

    loop {
        let record = tokio::select! {
            record = receiver.recv() => record,
            true = async { shutdown.wait_for(|stopping| *stopping).await.is_ok() } => {
                // Stop accepting records, then write the ones already queued.
                receiver.close();
                receiver.recv().await
            }
        };
        let Some(record) = record else {
            break;
        };

        if current_date != record.date {
            flush(&mut file, logs_dir).await;
            match open_log_file(logs_dir, &record.date).await {
                Ok(next_file) => {
                    current_date.clone_from(&record.date);
                    file = Some(BufWriter::new(next_file));
                }
                Err(error) => {
                    tracing::error!(error = %format_args!("{error:#}"), %logs_dir, "failed to rotate message log");
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
                tracing::error!(error = %format_args!("{error:#}"), %logs_dir, "failed to encode message log record");
                continue;
            }
        };
        line.push(b'\n');
        if let Err(error) = active_file.write_all(&line).await {
            tracing::error!(error = %format_args!("{error:#}"), %logs_dir, "failed to append message log record");
            file = None;
            current_date.clear();
            continue;
        }
        // Batch writes during bursts, but reach the disk once the queue is idle.
        if receiver.is_empty() && !flush(&mut file, logs_dir).await {
            current_date.clear();
        }
    }
    flush(&mut file, logs_dir).await;
}

/// Flushes the open log file. Returns false and closes it when the write fails.
async fn flush(file: &mut Option<BufWriter<File>>, logs_dir: &str) -> bool {
    let Some(active_file) = file.as_mut() else {
        return true;
    };
    if let Err(error) = active_file.flush().await {
        tracing::error!(error = %format_args!("{error:#}"), %logs_dir, "failed to flush message log");
        *file = None;
        return false;
    }
    true
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

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};
    use chrono::TimeZone as _;
    use chrono_tz::Europe::Berlin;
    use poise::serenity_prelude as serenity;
    use tokio::sync::{mpsc, watch};

    use super::{LogRecord, run_writer};

    fn record(content: &str) -> LogRecord {
        LogRecord {
            timestamp: "2026-11-23T12:00:00+01:00".to_owned(),
            channel: "general".to_owned(),
            author: "user".to_owned(),
            content: content.to_owned(),
            message_id: "1".to_owned(),
            channel_id: "2".to_owned(),
            guild_id: Some("3".to_owned()),
            user_id: "4".to_owned(),
            date: "2026-11-23".to_owned(),
        }
    }

    #[tokio::test]
    async fn writes_queued_records_before_shutting_down() -> Result<()> {
        let logs_dir = std::env::temp_dir().join(format!(
            "steward-log-test-{}-{}",
            std::process::id(),
            line!()
        ));
        tokio::fs::create_dir_all(&logs_dir).await?;
        let logs_dir_str: &'static str = Box::leak(logs_dir.display().to_string().into_boxed_str());

        let (sender, receiver) = mpsc::channel(8);
        let (shutdown, shutdown_receiver) = watch::channel(false);
        // Queue every record before the writer runs, then ask it to stop at once.
        for content in ["first", "second", "third"] {
            sender.try_send(record(content))?;
        }
        shutdown.send_replace(true);
        run_writer(logs_dir_str, receiver, shutdown_receiver).await;

        let written = tokio::fs::read_to_string(logs_dir.join("2026-11-23.jsonl")).await;
        tokio::fs::remove_dir_all(&logs_dir).await?;
        let contents = written?
            .lines()
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| value["content"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        ensure!(
            contents == ["first", "second", "third"],
            "unexpected log: {contents:?}"
        );
        Ok(())
    }

    #[test]
    fn records_identify_the_original_message() -> Result<()> {
        let mut message = serenity::Message::default();
        message.id = serenity::MessageId::new(1_234_567_890_123_456_789);
        message.channel_id = serenity::ChannelId::new(222);
        message.guild_id = Some(serenity::GuildId::new(333));
        message.author.id = serenity::UserId::new(444);
        message.author.name = "alice".to_owned();
        message.content = "hello".to_owned();
        let now = Berlin
            .with_ymd_and_hms(2026, 11, 23, 23, 30, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("invalid test time"))?;

        let record = LogRecord::new(&message, "general", now);
        let value = serde_json::to_value(&record)?;

        ensure!(
            value
                == serde_json::json!({
                    "timestamp": "2026-11-23T23:30:00+01:00",
                    "channel": "general",
                    "author": "alice",
                    "content": "hello",
                    "message_id": "1234567890123456789",
                    "channel_id": "222",
                    "guild_id": "333",
                    "user_id": "444",
                }),
            "unexpected record: {value}"
        );
        ensure!(
            record.date == "2026-11-23",
            "unexpected date {}",
            record.date
        );
        Ok(())
    }
}
