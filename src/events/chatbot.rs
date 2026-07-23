use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, LazyLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use poise::serenity_prelude as serenity;
use serenity::Mentionable as _;
use tokio::sync::Mutex;

use crate::{ai::ChatMessage, state::AppState};

const CLEANUP_INTERVAL: Duration = Duration::from_mins(5);
const CONVERSATION_IDLE_TTL: Duration = Duration::from_mins(30);
const GENERATION_DEBOUNCE: Duration = Duration::from_secs(1);
const MAX_CONTEXTS: usize = 2_048;
const MAX_HISTORY_BYTES: usize = 16 * 1024;
const MAX_HISTORY_MESSAGES: usize = 24;

static SERVICE_STARTED_AT: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Clone)]
pub struct ChatbotService {
    contexts: Arc<Mutex<HashMap<ConversationKey, Arc<ConversationHandle>>>>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ConversationKey {
    bot: &'static str,
    channel: serenity::ChannelId,
    user: serenity::UserId,
}

struct ConversationHandle {
    last_activity: AtomicU64,
    state: Mutex<Conversation>,
}

struct Conversation {
    generating: bool,
    messages: VecDeque<HistoryEntry>,
    next_order: u64,
    pending: bool,
    reply_target: ReplyTarget,
    scheduled: bool,
}

struct HistoryEntry {
    message: ChatMessage,
    order: u64,
}

#[derive(Clone, Copy)]
struct ReplyTarget {
    author: serenity::UserId,
    channel: serenity::ChannelId,
    message: serenity::MessageId,
}

impl ReplyTarget {
    fn from_message(message: &serenity::Message) -> Self {
        Self {
            author: message.author.id,
            channel: message.channel_id,
            message: message.id,
        }
    }
}

impl ConversationHandle {
    fn new(reply_target: ReplyTarget) -> Self {
        Self {
            last_activity: AtomicU64::new(activity_tick()),
            state: Mutex::new(Conversation {
                generating: false,
                messages: VecDeque::new(),
                next_order: 0,
                pending: false,
                reply_target,
                scheduled: false,
            }),
        }
    }

    fn touch(&self) {
        self.last_activity.store(activity_tick(), Ordering::Relaxed);
    }
}

impl Default for ChatbotService {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatbotService {
    pub fn new() -> Self {
        let contexts = Arc::new(Mutex::new(HashMap::new()));
        start_cleanup_task(Arc::downgrade(&contexts));
        Self { contexts }
    }

    async fn queue(
        &self,
        ctx: serenity::Context,
        data: AppState,
        reply_target: ReplyTarget,
        content: Arc<str>,
    ) -> bool {
        let key = ConversationKey {
            bot: data.bot.id,
            channel: reply_target.channel,
            user: reply_target.author,
        };
        let conversation = {
            let mut contexts = self.contexts.lock().await;
            if !contexts.contains_key(&key) && contexts.len() >= MAX_CONTEXTS {
                evict_oldest_idle_context(&mut contexts);
            }
            if !contexts.contains_key(&key) && contexts.len() >= MAX_CONTEXTS {
                tracing::warn!(
                    bot = data.bot.id,
                    channel_id = %reply_target.channel,
                    "chatbot context limit reached"
                );
                return false;
            }
            Arc::clone(
                contexts
                    .entry(key)
                    .or_insert_with(|| Arc::new(ConversationHandle::new(reply_target))),
            )
        };

        conversation.touch();
        let should_schedule = {
            let mut state = conversation.state.lock().await;
            state.next_order = state.next_order.saturating_add(2);
            let order = state.next_order;
            state.messages.push_back(HistoryEntry {
                message: ChatMessage::User(content),
                order,
            });
            trim_history(&mut state.messages);
            state.reply_target = reply_target;
            if state.generating {
                state.pending = true;
                false
            } else if state.scheduled {
                false
            } else {
                state.scheduled = true;
                true
            }
        };
        if should_schedule {
            schedule_generation(ctx, data, conversation);
        }
        true
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
    let reply_target = ReplyTarget::from_message(message);
    if data
        .services
        .patterns
        .is_prompt_injection(data.bot, content)
    {
        send_reply(
            ctx,
            reply_target,
            data.bot.chatbot.prompt_injection_error_message,
            false,
        )
        .await?;
        return Ok(());
    }

    let queued = data
        .services
        .chatbot
        .queue(ctx.clone(), data.clone(), reply_target, Arc::from(content))
        .await;
    if !queued {
        send_reply(
            ctx,
            reply_target,
            data.bot.chatbot.generation_error_message,
            false,
        )
        .await?;
    }
    Ok(())
}

fn schedule_generation(
    ctx: serenity::Context,
    data: AppState,
    conversation: Arc<ConversationHandle>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(GENERATION_DEBOUNCE).await;
        let (messages, reply_target, request_order) = {
            let mut state = conversation.state.lock().await;
            if state.generating || !state.scheduled {
                return;
            }
            state.generating = true;
            state.pending = false;
            state.scheduled = false;
            (
                state
                    .messages
                    .iter()
                    .map(|entry| entry.message.clone())
                    .collect::<Vec<_>>(),
                state.reply_target,
                state.messages.back().map_or(0, |entry| entry.order),
            )
        };

        let _ = reply_target.channel.broadcast_typing(&ctx.http).await;
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
                    let _ = reply_target.channel.broadcast_typing(&ctx.http).await;
                }
            }
        };

        let assistant_message = match generated {
            Ok(text) => match send_reply(&ctx, reply_target, &text, true).await {
                Ok(true) => Some(ChatMessage::Assistant(text.into())),
                Ok(false) => None,
                Err(error) => {
                    tracing::error!(%error, "failed to send chatbot reply");
                    None
                }
            },
            Err(error) => {
                tracing::error!(%error, bot = data.bot.id, "support response generation failed");
                let fallback = data.bot.chatbot.generation_error_message;
                match send_reply(&ctx, reply_target, fallback, true).await {
                    Ok(true) => Some(ChatMessage::Assistant(Arc::from(fallback))),
                    Ok(false) => None,
                    Err(send_error) => {
                        tracing::error!(%send_error, "failed to send chatbot error reply");
                        None
                    }
                }
            }
        };

        let schedule_next = {
            let mut state = conversation.state.lock().await;
            if let Some(message) = assistant_message {
                insert_reply(&mut state.messages, request_order, message);
                trim_history(&mut state.messages);
            }
            state.generating = false;
            if state.pending {
                state.pending = false;
                state.scheduled = true;
                true
            } else {
                false
            }
        };
        if schedule_next {
            schedule_generation(ctx, data, conversation);
        }
    });
}

fn insert_reply(messages: &mut VecDeque<HistoryEntry>, request_order: u64, message: ChatMessage) {
    if !messages.iter().any(|entry| entry.order == request_order) {
        return;
    }
    let entry = HistoryEntry {
        message,
        order: request_order.saturating_add(1),
    };
    let index = messages
        .iter()
        .position(|candidate| candidate.order > entry.order)
        .unwrap_or(messages.len());
    messages.insert(index, entry);
}

fn trim_history(messages: &mut VecDeque<HistoryEntry>) {
    let mut total_bytes = messages
        .iter()
        .map(|entry| entry.message.len())
        .sum::<usize>();
    while messages.len() > MAX_HISTORY_MESSAGES || total_bytes > MAX_HISTORY_BYTES {
        let Some(removed) = messages.pop_front() else {
            break;
        };
        total_bytes = total_bytes.saturating_sub(removed.message.len());

        while matches!(
            messages.front(),
            Some(HistoryEntry {
                message: ChatMessage::Assistant(_),
                ..
            })
        ) {
            if let Some(removed) = messages.pop_front() {
                total_bytes = total_bytes.saturating_sub(removed.message.len());
            }
        }
    }
}

fn activity_tick() -> u64 {
    u64::try_from(SERVICE_STARTED_AT.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn start_cleanup_task(contexts: Weak<Mutex<HashMap<ConversationKey, Arc<ConversationHandle>>>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            let Some(contexts) = contexts.upgrade() else {
                return;
            };
            let now = activity_tick();
            let ttl = u64::try_from(CONVERSATION_IDLE_TTL.as_millis()).unwrap_or(u64::MAX);
            contexts.lock().await.retain(|_, conversation| {
                Arc::strong_count(conversation) > 1
                    || now.saturating_sub(conversation.last_activity.load(Ordering::Relaxed)) < ttl
            });
        }
    });
}

fn evict_oldest_idle_context(contexts: &mut HashMap<ConversationKey, Arc<ConversationHandle>>) {
    let oldest = contexts
        .iter()
        .filter(|(_, conversation)| Arc::strong_count(conversation) == 1)
        .min_by_key(|(_, conversation)| conversation.last_activity.load(Ordering::Relaxed))
        .map(|(key, _)| *key);
    if let Some(key) = oldest {
        contexts.remove(&key);
    }
}

async fn send_reply(
    ctx: &serenity::Context,
    target: ReplyTarget,
    content: &str,
    fallback_to_channel: bool,
) -> Result<bool> {
    let mention = target.author.mention().to_string();
    let content = if content.starts_with(&mention) {
        content.to_owned()
    } else {
        format!("{mention} {content}")
    };
    let allowed_mentions = serenity::CreateAllowedMentions::new()
        .users([target.author])
        .replied_user(true);
    let reply = serenity::CreateMessage::new()
        .content(&content)
        .reference_message((target.channel, target.message))
        .allowed_mentions(allowed_mentions.clone());

    if target.channel.send_message(ctx, reply).await.is_ok() {
        return Ok(true);
    }
    if !fallback_to_channel {
        return Ok(false);
    }

    target
        .channel
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .content(content)
                .allowed_mentions(allowed_mentions.replied_user(false)),
        )
        .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Arc};

    use anyhow::{Result, ensure};

    use super::{
        HistoryEntry, MAX_HISTORY_BYTES, MAX_HISTORY_MESSAGES, insert_reply, trim_history,
    };
    use crate::ai::ChatMessage;

    #[test]
    fn trims_complete_old_turns_to_the_history_limits() -> Result<()> {
        let mut messages = VecDeque::new();
        for index in 0..40 {
            messages.push_back(HistoryEntry {
                message: ChatMessage::User(Arc::from(format!(
                    "question {index} {}",
                    "x".repeat(1_000)
                ))),
                order: index * 2,
            });
            messages.push_back(HistoryEntry {
                message: ChatMessage::Assistant(Arc::from(format!("answer {index}"))),
                order: index * 2 + 1,
            });
        }

        trim_history(&mut messages);

        ensure!(messages.len() <= MAX_HISTORY_MESSAGES);
        ensure!(
            messages
                .iter()
                .map(|entry| entry.message.len())
                .sum::<usize>()
                <= MAX_HISTORY_BYTES
        );
        ensure!(matches!(
            messages.front(),
            Some(HistoryEntry {
                message: ChatMessage::User(_),
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn inserts_a_reply_before_messages_queued_during_generation() -> Result<()> {
        let mut messages = VecDeque::from([
            HistoryEntry {
                message: ChatMessage::User(Arc::from("first")),
                order: 2,
            },
            HistoryEntry {
                message: ChatMessage::User(Arc::from("queued later")),
                order: 4,
            },
        ]);

        insert_reply(&mut messages, 2, ChatMessage::Assistant(Arc::from("reply")));

        ensure!(matches!(
            messages.get(1),
            Some(HistoryEntry {
                message: ChatMessage::Assistant(_),
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn omits_a_reply_when_its_request_was_trimmed() -> Result<()> {
        let mut messages = VecDeque::from([HistoryEntry {
            message: ChatMessage::User(Arc::from("queued later")),
            order: 4,
        }]);

        insert_reply(
            &mut messages,
            2,
            ChatMessage::Assistant(Arc::from("stale reply")),
        );

        ensure!(messages.len() == 1);
        ensure!(matches!(
            messages.front(),
            Some(HistoryEntry {
                message: ChatMessage::User(_),
                ..
            })
        ));
        Ok(())
    }
}
