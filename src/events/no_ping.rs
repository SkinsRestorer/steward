use anyhow::Result;
use poise::serenity_prelude as serenity;

use crate::{config::NoPingConfig, state::AppState};

const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
const REPLY_CONVERSATION_WINDOW_MS: i64 = 60 * 60 * 1_000;
const PARTICIPATION_WINDOW_MS: i64 = 24 * 60 * 60 * 1_000;
const RECENT_PARTICIPATION_GRACE_MS: i64 = 5 * 60 * 1_000;

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    if message.mentions.is_empty() && message.mention_roles.is_empty() {
        return Ok(());
    }
    let config = data.bot.no_ping;
    let sender_roles = message
        .member
        .as_deref()
        .map(|member| member.roles.as_slice())
        .unwrap_or_default();

    if has_configured_role(sender_roles, config.staff_role_ids)
        || has_configured_role(sender_roles, config.exempt_role_ids)
        || !mentions_staff(ctx, message, &config).await
        || should_skip_reply_warning(ctx, message).await
    {
        return Ok(());
    }

    message
        .reply(ctx, (config.warning_message)(message.author.id))
        .await?;
    Ok(())
}

fn has_configured_role(roles: &[serenity::RoleId], configured: &[u64]) -> bool {
    roles.iter().any(|role| configured.contains(&role.get()))
}

async fn mentions_staff(
    ctx: &serenity::Context,
    message: &serenity::Message,
    config: &NoPingConfig,
) -> bool {
    if has_configured_role(&message.mention_roles, config.staff_role_ids) {
        return true;
    }
    let Some(guild_id) = message.guild_id else {
        return false;
    };

    for user in &message.mentions {
        if let Ok(member) = guild_id.member(ctx, user.id).await
            && has_configured_role(&member.roles, config.staff_role_ids)
        {
            return true;
        }
    }
    false
}

async fn should_skip_reply_warning(ctx: &serenity::Context, message: &serenity::Message) -> bool {
    let Some(reference) = fetch_referenced_message(ctx, message).await else {
        return false;
    };
    if reply_target_was_reply_to_sender(ctx, message, &reference).await {
        return true;
    }

    let age = message.timestamp.timestamp_millis() - reference.timestamp.timestamp_millis();
    age <= REPLY_CONVERSATION_WINDOW_MS && sender_is_active_chat_participant(ctx, message).await
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
    let earliest_timestamp = timestamp - PARTICIPATION_WINDOW_MS;
    let latest_timestamp = timestamp - RECENT_PARTICIPATION_GRACE_MS;
    let mut before = create_snowflake_after_timestamp(latest_timestamp);

    loop {
        let messages = match message
            .channel_id
            .messages(ctx, serenity::GetMessages::new().before(before).limit(100))
            .await
        {
            Ok(messages) => messages,
            Err(error) => {
                tracing::debug!(%error, "failed to inspect channel participation history");
                return false;
            }
        };
        if messages.is_empty() {
            return false;
        }
        if messages.iter().any(|candidate| {
            candidate.author.id == message.author.id
                && candidate.timestamp.timestamp_millis() >= earliest_timestamp
        }) {
            return true;
        }

        let Some(oldest) = messages.iter().min_by_key(|candidate| candidate.timestamp) else {
            return false;
        };
        if oldest.timestamp.timestamp_millis() < earliest_timestamp {
            return false;
        }
        before = oldest.id;
    }
}

fn create_snowflake_after_timestamp(timestamp_ms: i64) -> serenity::MessageId {
    let timestamp = timestamp_ms.max(DISCORD_EPOCH_MS) + 1;
    let value = u64::try_from(timestamp - DISCORD_EPOCH_MS).unwrap_or_default() << 22;
    serenity::MessageId::new(value)
}

#[cfg(test)]
mod tests {
    use super::{DISCORD_EPOCH_MS, create_snowflake_after_timestamp, has_configured_role};
    use poise::serenity_prelude as serenity;

    #[test]
    fn creates_a_snowflake_strictly_after_the_timestamp() {
        let timestamp = DISCORD_EPOCH_MS + 123_456;
        let snowflake = create_snowflake_after_timestamp(timestamp);
        let encoded_timestamp = i64::try_from(snowflake.get() >> 22).unwrap() + DISCORD_EPOCH_MS;

        assert_eq!(encoded_timestamp, timestamp + 1);
    }

    #[test]
    fn detects_configured_roles() {
        assert!(has_configured_role(&[serenity::RoleId::new(7)], &[3, 7, 9]));
    }
}
