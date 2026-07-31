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
    scheduled_order: Option<u64>,
}

struct GenerationRequest {
    messages: Vec<ChatMessage>,
    reply_target: ReplyTarget,
    request_order: u64,
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
            state: Mutex::new(Conversation::new(reply_target)),
        }
    }

    fn touch(&self) {
        self.last_activity.store(activity_tick(), Ordering::Relaxed);
    }
}

impl Conversation {
    fn new(reply_target: ReplyTarget) -> Self {
        Self {
            generating: false,
            messages: VecDeque::new(),
            next_order: 0,
            pending: false,
            reply_target,
            scheduled_order: None,
        }
    }

    fn enqueue(&mut self, reply_target: ReplyTarget, content: Arc<str>) -> Option<u64> {
        self.next_order = self.next_order.saturating_add(2);
        let order = self.next_order;
        self.messages.push_back(HistoryEntry {
            message: ChatMessage::User(content),
            order,
        });
        trim_history(&mut self.messages);
        self.reply_target = reply_target;

        if self.generating {
            self.pending = true;
            return None;
        }

        self.scheduled_order = Some(order);
        Some(order)
    }

    fn begin_generation(&mut self, scheduled_order: u64) -> Option<GenerationRequest> {
        if self.generating || self.scheduled_order != Some(scheduled_order) {
            return None;
        }

        self.generating = true;
        self.pending = false;
        self.scheduled_order = None;
        Some(GenerationRequest {
            messages: self
                .messages
                .iter()
                .map(|entry| entry.message.clone())
                .collect(),
            reply_target: self.reply_target,
            request_order: self.messages.back().map_or(0, |entry| entry.order),
        })
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
        let scheduled_order = {
            let mut state = conversation.state.lock().await;
            state.enqueue(reply_target, content)
        };
        if let Some(scheduled_order) = scheduled_order {
            schedule_generation(ctx, data, conversation, scheduled_order);
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
    scheduled_order: u64,
) {
    tokio::spawn(async move {
        tokio::time::sleep(GENERATION_DEBOUNCE).await;
        let Some(request) = ({
            let mut state = conversation.state.lock().await;
            state.begin_generation(scheduled_order)
        }) else {
            return;
        };

        let _ = request
            .reply_target
            .channel
            .broadcast_typing(&ctx.http)
            .await;
        let ai = data.services.ai.clone();
        let generation = ai.generate_response(
            &request.messages,
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
                    let _ = request.reply_target.channel.broadcast_typing(&ctx.http).await;
                }
            }
        };

        let assistant_message = match generated {
            Ok(text) => match send_reply(&ctx, request.reply_target, &text, true).await {
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
                match send_reply(&ctx, request.reply_target, fallback, true).await {
                    Ok(true) => Some(ChatMessage::Assistant(Arc::from(fallback))),
                    Ok(false) => None,
                    Err(send_error) => {
                        tracing::error!(%send_error, "failed to send chatbot error reply");
                        None
                    }
                }
            }
        };

        let next_scheduled_order = {
            let mut state = conversation.state.lock().await;
            if let Some(message) = assistant_message {
                insert_reply(&mut state.messages, request.request_order, message);
                trim_history(&mut state.messages);
            }
            state.generating = false;
            if state.pending {
                state.pending = false;
                state.scheduled_order = Some(state.next_order);
                Some(state.next_order)
            } else {
                None
            }
        };
        if let Some(next_scheduled_order) = next_scheduled_order {
            schedule_generation(ctx, data, conversation, next_scheduled_order);
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
        Conversation, HistoryEntry, MAX_HISTORY_BYTES, MAX_HISTORY_MESSAGES, ReplyTarget,
        insert_reply, trim_history,
    };
    use crate::ai::ChatMessage;
    use poise::serenity_prelude as serenity;

    fn reply_target(message: u64) -> ReplyTarget {
        ReplyTarget {
            author: serenity::UserId::new(1),
            channel: serenity::ChannelId::new(2),
            message: serenity::MessageId::new(message),
        }
    }

    #[test]
    fn debounces_generation_until_the_latest_message() -> Result<()> {
        let mut conversation = Conversation::new(reply_target(1));
        let first_order = conversation
            .enqueue(reply_target(1), Arc::from("first"))
            .ok_or_else(|| anyhow::anyhow!("first message was not scheduled"))?;
        let latest_order = conversation
            .enqueue(reply_target(2), Arc::from("second"))
            .ok_or_else(|| anyhow::anyhow!("second message was not scheduled"))?;

        ensure!(conversation.begin_generation(first_order).is_none());
        let request = conversation
            .begin_generation(latest_order)
            .ok_or_else(|| anyhow::anyhow!("latest message was not scheduled"))?;
        ensure!(request.messages.len() == 2);
        ensure!(request.reply_target.message == serenity::MessageId::new(2));
        Ok(())
    }

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
