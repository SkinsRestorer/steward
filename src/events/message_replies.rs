use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::{
    config::{MessageReply, ReplyTrigger},
    state::AppState,
};

/// A user who repeats the same mistake gets the same reply at most this often.
const REPLY_COOLDOWN: Duration = Duration::from_mins(15);

/// Bot ID, user, and reply index mapped to when the reply was last sent.
type ReplyKey = (&'static str, serenity::UserId, usize);

static RECENT_REPLIES: LazyLock<Mutex<HashMap<ReplyKey, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    for (index, reply) in data.bot.message_replies.iter().enumerate() {
        if !matches(reply.trigger, &message.content) {
            continue;
        }
        let claimed = claim_reply(
            &mut RECENT_REPLIES
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
            (data.bot.id, message.author.id, index),
            Instant::now(),
        );
        if claimed {
            send_reply(ctx, data, message, reply).await?;
        }
    }
    Ok(())
}

/// Records a reply and returns `true` unless the same reply went to the same
/// user within the cooldown. Expired entries are removed on each call.
fn claim_reply(recent: &mut HashMap<ReplyKey, Instant>, key: ReplyKey, now: Instant) -> bool {
    recent.retain(|_, sent_at| now.saturating_duration_since(*sent_at) < REPLY_COOLDOWN);
    if recent.contains_key(&key) {
        return false;
    }
    recent.insert(key, now);
    true
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
    use std::{collections::HashMap, time::Instant};

    use anyhow::{Result, ensure};
    use poise::serenity_prelude as serenity;

    use super::{REPLY_COOLDOWN, claim_reply, matches};
    use crate::config::ReplyTrigger;

    #[test]
    fn repeats_a_reply_to_a_user_only_after_the_cooldown() -> Result<()> {
        let mut recent = HashMap::new();
        let start = Instant::now();
        let user = serenity::UserId::new(1);
        let other_user = serenity::UserId::new(2);
        ensure!(claim_reply(&mut recent, ("steward", user, 0), start));
        ensure!(!claim_reply(&mut recent, ("steward", user, 0), start));
        // Other replies, users, and bots are independent.
        ensure!(claim_reply(&mut recent, ("steward", user, 1), start));
        ensure!(claim_reply(&mut recent, ("steward", other_user, 0), start));
        ensure!(claim_reply(&mut recent, ("jarvis", user, 0), start));
        let almost = start + REPLY_COOLDOWN / 2;
        ensure!(!claim_reply(&mut recent, ("steward", user, 0), almost));
        ensure!(claim_reply(
            &mut recent,
            ("steward", user, 0),
            start + REPLY_COOLDOWN
        ));
        Ok(())
    }

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
