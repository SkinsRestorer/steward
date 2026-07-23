use anyhow::Result;
use poise::serenity_prelude as serenity;

use crate::state::AppState;

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    thread: &serenity::GuildChannel,
) -> Result<()> {
    let Some(parent_id) = thread.parent_id else {
        return Ok(());
    };
    let parent_is_forum = ctx.cache.guild(thread.guild_id).is_some_and(|guild| {
        guild
            .channels
            .get(&parent_id)
            .is_some_and(|parent| parent.kind == serenity::ChannelType::Forum)
    });
    if !parent_is_forum {
        return Ok(());
    }

    thread.id.join_thread(&ctx.http).await?;
    let starter = thread
        .id
        .message(&ctx.http, serenity::MessageId::new(thread.id.get()))
        .await?;
    let config = data.bot.thread_starter;
    let support = serenity::CreateEmbed::new()
        .colour(data.bot.accent_color)
        .title(config.support_title)
        .image(config.support_banner_url)
        .description(config.support_description);
    let priority = serenity::CreateEmbed::new()
        .colour(data.bot.accent_color)
        .title(config.priority_title)
        .image(config.priority_banner_url)
        .description(config.priority_description);
    let support_links = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new_link(config.support_gpt_url).label("🤖 Open Support GPT"),
        serenity::CreateButton::new_link(config.docs_url).label("📚 Open Documentation"),
    ]);
    let priority_links = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new_link(config.pricing_url).label("💳 View Pricing"),
        serenity::CreateButton::new_link(config.priority_support_url).label("☕ Join via Ko-fi"),
    ]);

    starter
        .channel_id
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new()
                .embeds(vec![support, priority])
                .components(vec![support_links, priority_links])
                .reference_message(&starter),
        )
        .await?;
    Ok(())
}
