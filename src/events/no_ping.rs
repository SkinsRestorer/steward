use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::{config::NoPingConfig, state::AppState};

const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
const REPLY_CONVERSATION_WINDOW_MS: i64 = 60 * 60 * 1_000;
const PARTICIPATION_WINDOW_MS: i64 = 24 * 60 * 60 * 1_000;
const RECENT_PARTICIPATION_GRACE_MS: i64 = 5 * 60 * 1_000;
// Bound the REST calls one staff ping can cost in a busy channel.
const MAX_HISTORY_PAGES: usize = 3;
const HISTORY_PAGE_SIZE: u8 = 100;
// One warning is enough. Repeating it for every ping floods support channels.
const WARNING_COOLDOWN: Duration = Duration::from_mins(10);

static WARNING_COOLDOWNS: LazyLock<Mutex<WarningCooldowns>> =
    LazyLock::new(|| Mutex::new(WarningCooldowns::default()));

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    if message.mentions.is_empty() && message.mention_roles.is_empty() {
        return Ok(());
    }
    let Some(config) = data.bot.no_ping else {
        return Ok(());
    };
    let sender_roles = message
        .member
        .as_deref()
        .map(|member| member.roles.as_slice())
        .unwrap_or_default();

    if has_configured_role(sender_roles, config.staff_role_ids)
        || has_configured_role(sender_roles, config.exempt_role_ids)
    {
        return Ok(());
    }
    let staff_role_ping = has_configured_role(&message.mention_roles, config.staff_role_ids);
    let staff_users = mentioned_staff_users(ctx, message, &config).await;
    if !staff_role_ping && staff_users.is_empty() {
        return Ok(());
    }
    if !staff_role_ping && is_conversation_reply(ctx, message, &staff_users).await {
        return Ok(());
    }

    let key = (data.bot.id, message.channel_id, message.author.id);
    if !lock_cooldowns().try_claim(key, Instant::now()) {
        return Ok(());
    }
    let sent = message
        .channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .content((config.warning_message)(message.author.id))
                .reference_message(message)
                .allowed_mentions(
                    serenity::CreateAllowedMentions::new().users([message.author.id]),
                ),
        )
        .await;
    if sent.is_err() {
        // Let the next ping try again when this warning never reached the user.
        lock_cooldowns().release(key);
    }
    sent.context("failed to send the no-ping warning")?;
    Ok(())
}

type CooldownKey = (&'static str, serenity::ChannelId, serenity::UserId);

fn lock_cooldowns() -> std::sync::MutexGuard<'static, WarningCooldowns> {
    WARNING_COOLDOWNS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug, Default)]
struct WarningCooldowns {
    warned_at: HashMap<CooldownKey, Instant>,
}

impl WarningCooldowns {
    /// Returns whether a warning may be sent now and records it if so.
    fn try_claim(&mut self, key: CooldownKey, now: Instant) -> bool {
        self.warned_at
            .retain(|_, warned_at| now.saturating_duration_since(*warned_at) < WARNING_COOLDOWN);
        if self.warned_at.contains_key(&key) {
            return false;
        }
        self.warned_at.insert(key, now);
        true
    }

    fn release(&mut self, key: CooldownKey) {
        self.warned_at.remove(&key);
    }
}

fn has_configured_role(roles: &[serenity::RoleId], configured: &[u64]) -> bool {
    roles.iter().any(|role| configured.contains(&role.get()))
}

async fn mentioned_staff_users(
    ctx: &serenity::Context,
    message: &serenity::Message,
    config: &NoPingConfig,
) -> Vec<serenity::UserId> {
    let Some(guild_id) = message.guild_id else {
        return Vec::new();
    };

    let mut staff = Vec::new();
    for user in &message.mentions {
        // The gateway includes mention roles, so most mentions need no lookup.
        let is_staff = if let Some(member) = &user.member {
            has_configured_role(&member.roles, config.staff_role_ids)
        } else {
            match guild_id.member(ctx, user.id).await {
                Ok(member) => has_configured_role(&member.roles, config.staff_role_ids),
                Err(error) => {
                    tracing::debug!(
                        error = %format_args!("{error:#}"),
                        user_id = %user.id,
                        "failed to look up mentioned member"
                    );
                    false
                }
            }
        };
        if is_staff {
            staff.push(user.id);
        }
    }
    staff
}

/// A reply pings the replied-to author. Allow that ping in an active
/// conversation, but still warn when other staff members are mentioned.
async fn is_conversation_reply(
    ctx: &serenity::Context,
    message: &serenity::Message,
    staff_users: &[serenity::UserId],
) -> bool {
    let Some(reference) = fetch_referenced_message(ctx, message).await else {
        return false;
    };
    if !only_pings_reply_target(staff_users, reference.author.id) {
        return false;
    }
    if reply_target_was_reply_to_sender(ctx, message, &reference).await {
        return true;
    }

    let age = message
        .timestamp
        .timestamp_millis()
        .saturating_sub(reference.timestamp.timestamp_millis());
    age <= REPLY_CONVERSATION_WINDOW_MS && sender_is_active_chat_participant(ctx, message).await
}

fn only_pings_reply_target(
    staff_users: &[serenity::UserId],
    reply_author: serenity::UserId,
) -> bool {
    staff_users.iter().all(|user| *user == reply_author)
}

async fn reply_target_was_reply_to_sender(
    ctx: &serenity::Context,
    message: &serenity::Message,
    reference: &serenity::Message,
) -> bool {
    fetch_referenced_message(ctx, reference)
        .await
        .is_some_and(|target| target.author.id == message.author.id)
}

async fn fetch_referenced_message(
    ctx: &serenity::Context,
    message: &serenity::Message,
) -> Option<serenity::Message> {
    if let Some(reference) = &message.referenced_message {
        return Some((**reference).clone());
    }

    let reference = message.message_reference.as_ref()?;
    let message_id = reference.message_id?;
    reference.channel_id.message(ctx, message_id).await.ok()
}

async fn sender_is_active_chat_participant(
    ctx: &serenity::Context,
    message: &serenity::Message,
) -> bool {
    let timestamp = message.timestamp.timestamp_millis();
    let earliest_timestamp = timestamp.saturating_sub(PARTICIPATION_WINDOW_MS);
    let latest_timestamp = timestamp.saturating_sub(RECENT_PARTICIPATION_GRACE_MS);
    let mut before = create_snowflake_after_timestamp(latest_timestamp);

    for _ in 0..MAX_HISTORY_PAGES {
        let messages = match message
            .channel_id
            .messages(
                ctx,
                serenity::GetMessages::new()
                    .before(before)
                    .limit(HISTORY_PAGE_SIZE),
            )
            .await
        {
            Ok(messages) => messages,
            Err(error) => {
                // Do not warn a user because Discord could not answer.
                tracing::warn!(
                    error = %format_args!("{error:#}"),
                    channel_id = %message.channel_id,
                    "failed to inspect channel participation history"
                );
                return true;
            }
        };
        match inspect_history_page(&messages, message.author.id, earliest_timestamp) {
            HistoryPage::Participated => return true,
            HistoryPage::Exhausted => return false,
            HistoryPage::Continue(oldest) => before = oldest,
        }
    }
    false
}

#[derive(Debug, PartialEq, Eq)]
enum HistoryPage {
    Participated,
    Exhausted,
    Continue(serenity::MessageId),
}

fn inspect_history_page(
    messages: &[serenity::Message],
    author: serenity::UserId,
    earliest_timestamp: i64,
) -> HistoryPage {
    if messages.iter().any(|candidate| {
        candidate.author.id == author
            && candidate.timestamp.timestamp_millis() >= earliest_timestamp
    }) {
        return HistoryPage::Participated;
    }
    // A short page means Discord has no older messages to return.
    if messages.len() < usize::from(HISTORY_PAGE_SIZE) {
        return HistoryPage::Exhausted;
    }
    let Some(oldest) = messages.iter().min_by_key(|candidate| candidate.timestamp) else {
        return HistoryPage::Exhausted;
    };
    if oldest.timestamp.timestamp_millis() < earliest_timestamp {
        return HistoryPage::Exhausted;
    }
    HistoryPage::Continue(oldest.id)
}

fn create_snowflake_after_timestamp(timestamp_ms: i64) -> serenity::MessageId {
    const TIMESTAMP_MULTIPLIER: u64 = 1 << 22;
    const MAX_TIMESTAMP: u64 = u64::MAX >> 22;

    let timestamp = timestamp_ms.max(DISCORD_EPOCH_MS).saturating_add(1);
    let elapsed = timestamp.saturating_sub(DISCORD_EPOCH_MS);
    let value = u64::try_from(elapsed).map_or(0, |elapsed| {
        elapsed
            .min(MAX_TIMESTAMP)
            .saturating_mul(TIMESTAMP_MULTIPLIER)
    });
    serenity::MessageId::new(value)
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use std::time::{Duration, Instant};

    use super::{
        DISCORD_EPOCH_MS, HISTORY_PAGE_SIZE, HistoryPage, WARNING_COOLDOWN, WarningCooldowns,
        create_snowflake_after_timestamp, has_configured_role, inspect_history_page,
        only_pings_reply_target,
    };
    use poise::serenity_prelude as serenity;

    #[test]
    fn warns_a_user_once_per_channel_until_the_cooldown_ends() -> Result<()> {
        let mut cooldowns = WarningCooldowns::default();
        let start = Instant::now();
        let channel = serenity::ChannelId::new(1);
        let user = serenity::UserId::new(2);
        let key = ("steward", channel, user);

        ensure!(cooldowns.try_claim(key, start));
        ensure!(!cooldowns.try_claim(key, start + Duration::from_secs(1)));
        ensure!(cooldowns.try_claim(("steward", serenity::ChannelId::new(3), user), start));
        ensure!(cooldowns.try_claim(("steward", channel, serenity::UserId::new(4)), start));
        ensure!(cooldowns.try_claim(("jarvis", channel, user), start));
        ensure!(cooldowns.try_claim(key, start + WARNING_COOLDOWN));
        Ok(())
    }

    #[test]
    fn releasing_a_claim_allows_a_retry() -> Result<()> {
        let mut cooldowns = WarningCooldowns::default();
        let now = Instant::now();
        let key = (
            "steward",
            serenity::ChannelId::new(1),
            serenity::UserId::new(2),
        );

        ensure!(cooldowns.try_claim(key, now));
        cooldowns.release(key);
        ensure!(cooldowns.try_claim(key, now));
        Ok(())
    }

    #[test]
    fn creates_a_snowflake_strictly_after_the_timestamp() -> Result<()> {
        let timestamp = DISCORD_EPOCH_MS.saturating_add(123_456);
        let snowflake = create_snowflake_after_timestamp(timestamp);
        let encoded_timestamp =
            i64::try_from(snowflake.get() >> 22)?.saturating_add(DISCORD_EPOCH_MS);

        ensure!(encoded_timestamp == timestamp.saturating_add(1));
        Ok(())
    }

    #[test]
    fn clamps_snowflakes_to_the_maximum_timestamp() -> Result<()> {
        let snowflake = create_snowflake_after_timestamp(i64::MAX);

        ensure!((snowflake.get() >> 22) == (u64::MAX >> 22));
        Ok(())
    }

    #[test]
    fn detects_configured_roles() -> Result<()> {
        ensure!(has_configured_role(&[serenity::RoleId::new(7)], &[3, 7, 9]));
        Ok(())
    }

    #[test]
    fn matches_only_configured_roles() -> Result<()> {
        let roles = [serenity::RoleId::new(8)];
        ensure!(!has_configured_role(&roles, &[3, 7, 9]));
        ensure!(!has_configured_role(&[], &[7]));
        ensure!(!has_configured_role(&[serenity::RoleId::new(7)], &[]));
        Ok(())
    }

    #[test]
    fn clamps_snowflakes_before_the_discord_epoch() -> Result<()> {
        // The earliest valid snowflake is one millisecond after the epoch.
        for timestamp in [i64::MIN, 0, DISCORD_EPOCH_MS - 1, DISCORD_EPOCH_MS] {
            ensure!(create_snowflake_after_timestamp(timestamp).get() == 1 << 22);
        }
        Ok(())
    }

    #[test]
    fn allows_only_the_reply_ping_to_skip_the_warning() -> Result<()> {
        let author = serenity::UserId::new(10);
        let other_staff = serenity::UserId::new(11);
        ensure!(only_pings_reply_target(&[author], author));
        ensure!(!only_pings_reply_target(&[author, other_staff], author));
        ensure!(!only_pings_reply_target(&[other_staff], author));
        Ok(())
    }

    fn history_message(id: u64, author: u64, timestamp_ms: i64) -> Result<serenity::Message> {
        let mut message = serenity::Message::default();
        message.id = serenity::MessageId::new(id);
        message.author.id = serenity::UserId::new(author);
        message.timestamp = serenity::Timestamp::from_millis(timestamp_ms)?;
        Ok(message)
    }

    fn full_page(author: u64, newest_ms: i64) -> Result<Vec<serenity::Message>> {
        (0..i64::from(HISTORY_PAGE_SIZE))
            .map(|offset| {
                history_message(
                    u64::try_from(1_000 - offset)?,
                    author,
                    newest_ms.saturating_sub(offset),
                )
            })
            .collect()
    }

    #[test]
    fn stops_paging_after_a_short_history_page() -> Result<()> {
        let sender = serenity::UserId::new(1);
        let page = [history_message(5, 2, 2_000_000_000_000)?];

        ensure!(inspect_history_page(&page, sender, 0) == HistoryPage::Exhausted);
        ensure!(inspect_history_page(&[], sender, 0) == HistoryPage::Exhausted);
        Ok(())
    }

    #[test]
    fn pages_to_the_oldest_message_of_a_full_page() -> Result<()> {
        let sender = serenity::UserId::new(1);
        let page = full_page(2, 2_000_000_000_000)?;
        let oldest = serenity::MessageId::new(1_000 - u64::from(HISTORY_PAGE_SIZE) + 1);

        ensure!(inspect_history_page(&page, sender, 0) == HistoryPage::Continue(oldest));
        ensure!(inspect_history_page(&page, sender, 2_000_000_000_000) == HistoryPage::Exhausted);
        Ok(())
    }

    #[test]
    fn finds_sender_participation_inside_the_window() -> Result<()> {
        let sender = serenity::UserId::new(1);
        let page = [history_message(5, 1, 2_000_000_000_000)?];

        ensure!(inspect_history_page(&page, sender, 0) == HistoryPage::Participated);
        ensure!(inspect_history_page(&page, sender, 2_000_000_000_001) == HistoryPage::Exhausted);
        Ok(())
    }
}
