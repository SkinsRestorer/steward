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
                .title("It looks like you're trying to spell SkinsRestorer!")
                .description(
                    "A useful tip to remember how to spell it is that we restore __many__ **SKINS**, not just one **SKIN**!",
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
                .title("Not in Discord you fool! Run it in the server 😄")
                .description(
                    "This is a server command, you run it in the server console or in the in-game chat, not in Discord!",
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
