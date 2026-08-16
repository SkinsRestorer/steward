use anyhow::Result;
use poise::serenity_prelude as serenity;

use crate::state::AppState;

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    if !data.bot.message_replies {
        return Ok(());
    }

    let lowercase = message.content.to_lowercase();
    let normalized = lowercase
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    if normalized.contains("skinrestorer") {
        reply_with_embed(
            ctx,
            message,
            serenity::CreateEmbed::new()
                .title("The name is SkinsRestorer")
                .description(
                    "Remember that SkinsRestorer restores many **skins**, so `Skins` is plural.",
                )
                .colour(data.bot.accent_color)
                .thumbnail("https://skinsrestorer.net/logo.png"),
        )
        .await?;
    }

    if lowercase.starts_with("/sr ") && lowercase.chars().filter(|char| *char == ' ').count() <= 1 {
        reply_with_embed(
            ctx,
            message,
            serenity::CreateEmbed::new()
                .title("Run this command on the Minecraft server")
                .description(
                    "Run this command in the server console or in-game chat. Discord cannot run server commands.",
                )
                .colour(data.bot.accent_color),
        )
        .await?;
    }

    Ok(())
}

async fn reply_with_embed(
    ctx: &serenity::Context,
    message: &serenity::Message,
    embed: serenity::CreateEmbed,
) -> Result<()> {
    message
        .channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .embed(embed)
                .reference_message(message),
        )
        .await?;
    Ok(())
}
