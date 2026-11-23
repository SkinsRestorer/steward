use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::state::AppState;

/// Only posts created this recently get the welcome message.
const MAX_THREAD_AGE_MS: i64 = 5 * 60 * 1_000;

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    thread: &serenity::GuildChannel,
) -> Result<()> {
    let Some(config) = data.bot.thread_starter else {
        return Ok(());
    };
    // The bot also gets ThreadCreate when it is added to an existing thread, for
    // example when someone mentions it in an old post.
    if thread.member.is_some()
        || !thread
            .parent_id
            .is_some_and(|parent_id| config.forum_ids.contains(&parent_id))
        || thread.owner_id == Some(ctx.cache.current_user().id)
        || !is_new_thread(thread.id, serenity::Timestamp::now())
    {
        return Ok(());
    }

    thread
        .id
        .join_thread(&ctx.http)
        .await
        .with_context(|| format!("failed to join forum post {}", thread.id))?;
    // One embed: what to post first, with paid support as a quiet last line.
    let embed = serenity::CreateEmbed::new()
        .colour(data.bot.accent_color)
        .title(config.support_title)
        .description(format!(
            "{}\n\n{}",
            config.support_description, config.priority_note
        ));
    let links = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new_link(config.docs_url).label("Documentation"),
        serenity::CreateButton::new_link(config.pricing_url).label("Pricing"),
        serenity::CreateButton::new_link(config.priority_support_url).label("Priority Support"),
    ]);

    // A forum post's starter message shares the thread's ID. It can arrive after
    // ThreadCreate, so reference it without failing when it is not there yet.
    let starter =
        serenity::MessageReference::from((thread.id, serenity::MessageId::new(thread.id.get())))
            .fail_if_not_exists(false);
    thread
        .id
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new()
                .embed(embed)
                .components(vec![links])
                .reference_message(starter),
        )
        .await
        .with_context(|| format!("failed to post the welcome message in {}", thread.id))?;
    Ok(())
}

/// A thread's ID records when it was created, so an old post that only
/// reached the bot now is not mistaken for a new one.
fn is_new_thread(thread_id: serenity::ChannelId, now: serenity::Timestamp) -> bool {
    now.unix_timestamp().saturating_mul(1_000).saturating_sub(
        thread_id
            .created_at()
            .unix_timestamp()
            .saturating_mul(1_000),
    ) <= MAX_THREAD_AGE_MS
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};
    use poise::serenity_prelude as serenity;

    use super::is_new_thread;

    #[test]
    fn welcomes_only_recently_created_threads() -> Result<()> {
        // Times are relative to when the thread ID says it was created.
        let thread = serenity::ChannelId::new(1_393_506_815_085_641_760);
        let created = thread.created_at().unix_timestamp();
        let at = |seconds: i64| serenity::Timestamp::from_unix_timestamp(created + seconds);
        ensure!(is_new_thread(thread, at(0)?));
        ensure!(is_new_thread(thread, at(5 * 60)?));
        ensure!(!is_new_thread(thread, at(5 * 60 + 1)?));
        ensure!(!is_new_thread(thread, at(30 * 24 * 60 * 60)?));
        Ok(())
    }
}
