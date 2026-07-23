use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use poise::serenity_prelude as serenity;
use serenity::Mentionable as _;
use tokio::sync::Mutex;

use crate::{
    ai::{AiService, ChatMessage},
    state::AppState,
};

#[derive(Clone, Default)]
pub struct ChatbotService {
    contexts: Arc<Mutex<HashMap<String, Arc<Mutex<Conversation>>>>>,
}

struct Conversation {
    generating: bool,
    last_message: serenity::Message,
    messages: Vec<ChatMessage>,
    pending: bool,
    revision: u64,
}

impl ChatbotService {
    pub fn new() -> Self {
        Self::default()
    }

    async fn queue(
        &self,
        ctx: serenity::Context,
        data: AppState,
        message: serenity::Message,
        content: String,
    ) {
        let key = format!(
            "{}:{}:{}",
            data.bot.id, message.channel_id, message.author.id
        );
        let conversation = {
            let mut contexts = self.contexts.lock().await;
            Arc::clone(contexts.entry(key).or_insert_with(|| {
                Arc::new(Mutex::new(Conversation {
                    generating: false,
                    last_message: message.clone(),
                    messages: Vec::new(),
                    pending: false,
                    revision: 0,
                }))
            }))
        };

        let revision = {
            let mut conversation = conversation.lock().await;
            conversation.messages.push(ChatMessage::User(content));
            conversation.last_message = message;
            conversation.revision = conversation.revision.saturating_add(1);
            if conversation.generating {
                conversation.pending = true;
                return;
            }
            conversation.revision
        };
        schedule_generation(ctx, data, conversation, revision);
    }
}

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    channel_name: Option<&str>,
) -> Result<()> {
    let Some(channel_name) = channel_name else {
        return Ok(());
    };
    if !data
        .bot
        .chatbot
        .channel_name_prefixes
        .iter()
        .any(|prefix| channel_name.starts_with(prefix))
    {
        return Ok(());
    }

    let content = message.content.trim();
    if content.is_empty() {
        return Ok(());
    }
    if AiService::is_prompt_injection_attempt(content, data.bot.chatbot.ai) {
        send_reply(
            ctx,
            message,
            data.bot.chatbot.prompt_injection_error_message,
            false,
        )
        .await?;
        return Ok(());
    }

    data.services
        .chatbot
        .queue(
            ctx.clone(),
            data.clone(),
            message.clone(),
            content.to_owned(),
        )
        .await;
    Ok(())
}

fn schedule_generation(
    ctx: serenity::Context,
    data: AppState,
    conversation: Arc<Mutex<Conversation>>,
    revision: u64,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let (messages, reply_message, insertion_index) = {
            let mut state = conversation.lock().await;
            if state.revision != revision || state.generating {
                return;
            }
            state.generating = true;
            state.pending = false;
            (
                state.messages.clone(),
                state.last_message.clone(),
                state.messages.len(),
            )
        };

        let _ = reply_message.channel_id.broadcast_typing(&ctx.http).await;
        let ai = data.services.ai.clone();
        let generation = ai.generate_response(
            &messages,
            data.bot.chatbot.ai,
            data.bot.chatbot.max_response_length,
        );
        tokio::pin!(generation);
        let mut typing_interval = tokio::time::interval(Duration::from_secs(8));
        typing_interval.tick().await;
        let generated = loop {
            tokio::select! {
                response = &mut generation => break response,
                _ = typing_interval.tick() => {
                    let _ = reply_message.channel_id.broadcast_typing(&ctx.http).await;
                }
            }
        };

        let assistant_message = match generated {
            Ok(text) => match send_reply(&ctx, &reply_message, &text, true).await {
                Ok(true) => Some(ChatMessage::Assistant(text)),
                Ok(false) => None,
                Err(error) => {
                    tracing::error!(%error, "failed to send chatbot reply");
                    None
                }
            },
            Err(error) => {
                tracing::error!(%error, bot = data.bot.id, "support response generation failed");
                let fallback = data.bot.chatbot.generation_error_message;
                match send_reply(&ctx, &reply_message, fallback, true).await {
                    Ok(true) => Some(ChatMessage::Assistant(fallback.to_owned())),
                    Ok(false) => None,
                    Err(send_error) => {
                        tracing::error!(%send_error, "failed to send chatbot error reply");
                        None
                    }
                }
            }
        };

        let next_revision = {
            let mut state = conversation.lock().await;
            if let Some(message) = assistant_message {
                let index = insertion_index.min(state.messages.len());
                state.messages.insert(index, message);
            }
            state.generating = false;
            if state.pending {
                state.pending = false;
                Some(state.revision)
            } else {
                None
            }
        };
        if let Some(next_revision) = next_revision {
            schedule_generation(ctx, data, conversation, next_revision);
        }
    });
}

async fn send_reply(
    ctx: &serenity::Context,
    message: &serenity::Message,
    content: &str,
    fallback_to_channel: bool,
) -> Result<bool> {
    let mention = message.author.mention().to_string();
    let content = if content.starts_with(&mention) {
        content.to_owned()
    } else {
        format!("{mention} {content}")
    };
    let allowed_mentions = serenity::CreateAllowedMentions::new()
        .users([message.author.id])
        .replied_user(true);
    let reply = serenity::CreateMessage::new()
        .content(&content)
        .reference_message(message)
        .allowed_mentions(allowed_mentions.clone());

    if message.channel_id.send_message(ctx, reply).await.is_ok() {
        return Ok(true);
    }
    if !fallback_to_channel {
        return Ok(false);
    }

    message
        .channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .content(content)
                .allowed_mentions(allowed_mentions.replied_user(false)),
        )
        .await?;
    Ok(true)
}
