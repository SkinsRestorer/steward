use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::{
    config::{MessageReply, ReplyTrigger},
    state::AppState,
};

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    for reply in data.bot.message_replies {
        if !matches(reply.trigger, &message.content) {
            continue;
        }
        send_reply(ctx, data, message, reply).await?;
    }
    Ok(())
}

fn matches(trigger: ReplyTrigger, content: &str) -> bool {
    match trigger {
        ReplyTrigger::NormalizedContains(needle) => content
            .to_lowercase()
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect::<String>()
            .contains(needle),
        ReplyTrigger::ServerCommand(command) => {
            let mut words = content.split(' ');
            words
                .next()
                .is_some_and(|first| first.eq_ignore_ascii_case(command))
                && words.next().is_some()
                && words.next().is_none()
        }
    }
}

async fn send_reply(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    reply: &MessageReply,
) -> Result<()> {
    let mut embed = serenity::CreateEmbed::new()
        .title(reply.title)
        .description(reply.description)
        .colour(data.bot.accent_color);
    if let Some(thumbnail_url) = reply.thumbnail_url {
        embed = embed.thumbnail(thumbnail_url);
    }
    message
        .channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .embed(embed)
                .reference_message(message)
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await
        .context("failed to send a canned reply")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::matches;
    use crate::config::ReplyTrigger;

    #[test]
    fn matches_misspelled_names_inside_punctuation() -> Result<()> {
        let trigger = ReplyTrigger::NormalizedContains("skinrestorer");
        ensure!(matches(trigger, "Is Skin-Restorer broken?"));
        ensure!(!matches(trigger, "SkinsRestorer works"));
        Ok(())
    }

    #[test]
    fn matches_server_commands_with_one_argument() -> Result<()> {
        let trigger = ReplyTrigger::ServerCommand("/sr");
        ensure!(matches(trigger, "/sr dump"));
        ensure!(matches(trigger, "/SR reload"));
        ensure!(!matches(trigger, "/sr"));
        ensure!(!matches(trigger, "/sr set skin Notch"));
        ensure!(!matches(trigger, "/srs dump"));
        Ok(())
    }
}
