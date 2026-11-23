use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::state::AppState;

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    thread: &serenity::GuildChannel,
) -> Result<()> {
    let Some(config) = data.bot.thread_starter else {
        return Ok(());
    };
    // The bot also gets ThreadCreate when it is added to an existing thread.
    if thread.member.is_some()
        || !thread
            .parent_id
            .is_some_and(|parent_id| config.forum_ids.contains(&parent_id))
        || thread.owner_id == Some(ctx.cache.current_user().id)
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
