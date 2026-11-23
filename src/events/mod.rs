mod autoupload;
pub mod chatbot;
mod checks;
pub(crate) mod honeypot;
pub(crate) mod logging;
mod message_replies;
mod no_ping;
mod paste;
mod thread_starter;

use poise::serenity_prelude as serenity;

use crate::state::{AppState, Error};

pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &AppState,
) -> Result<(), Error> {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "The bot only handles selected Discord gateway events"
    )]
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            tracing::info!(
                bot = data.bot.id,
                user = %data_about_bot.user.tag(),
                "Discord bot connected"
            );
        }
        serenity::FullEvent::GuildCreate { guild, .. }
            if !data.bot.allows_guild(Some(guild.id)) =>
        {
            tracing::warn!(
                bot = data.bot.id,
                guild_id = %guild.id,
                guild_name = %guild.name,
                "bot is in a server outside its allowlist and ignores it"
            );
        }
        serenity::FullEvent::Message { new_message } => {
            if data.bot.allows_guild(new_message.guild_id) {
                handle_message(ctx, data, new_message).await;
            }
        }
        serenity::FullEvent::ThreadCreate { thread }
            if data.bot.allows_guild(Some(thread.guild_id)) =>
        {
            if let Err(error) = thread_starter::handle(ctx, data, thread).await {
                tracing::error!(error = %format_args!("{error:#}"), bot = data.bot.id, "thread starter handler failed");
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_message(ctx: &serenity::Context, data: &AppState, message: &serenity::Message) {
    // System messages (thread created, member added, pins) are not user input.
    if message.author.bot
        || message.guild_id.is_none()
        || !matches!(
            message.kind,
            serenity::MessageType::Regular | serenity::MessageType::InlineReply
        )
    {
        return;
    }

    match honeypot::handle(ctx, data, message).await {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            tracing::error!(
                error = %format_args!("{error:#}"),
                bot = data.bot.id,
                guild_id = ?message.guild_id,
                channel_id = %message.channel_id,
                message_id = %message.id,
                user_id = %message.author.id,
                "honeypot handler failed"
            );
            // Only a confirmed trap channel stops processing. A failed lookup must
            // not drop moderation checks and replies for ordinary messages.
        }
    }

    let channel = message.guild_id.and_then(|guild_id| {
        ctx.cache.guild(guild_id).and_then(|guild| {
            guild
                .channels
                .get(&message.channel_id)
                .map(|channel| (channel.name.clone(), false))
                .or_else(|| {
                    guild
                        .threads
                        .iter()
                        .find(|thread| thread.id == message.channel_id)
                        .map(|thread| (thread.name.clone(), true))
                })
        })
    });
    let channel_name = channel.as_ref().map(|(name, _)| name.as_str());
    // Anyone who can create a thread picks its name, so only real channels can
    // turn on the chatbot.
    let chatbot_channel_name = channel
        .as_ref()
        .filter(|(_, is_thread)| !is_thread)
        .map(|(name, _)| name.as_str());
    let chatbot_channel = chatbot::supports_channel(data.bot, chatbot_channel_name);
    let log_result = logging::handle(data, message, channel_name);
    let upload = async {
        if chatbot_channel {
            Ok(())
        } else {
            autoupload::handle(ctx, data, message).await
        }
    };
    // The chatbot already answers in its channels, so canned replies would duplicate it.
    let replies = async {
        if chatbot_channel {
            Ok(())
        } else {
            message_replies::handle(ctx, data, message).await
        }
    };
    let (upload_result, chatbot_result, checks_result, replies_result, no_ping_result) = tokio::join!(
        upload,
        chatbot::handle(ctx, data, message, chatbot_channel_name),
        checks::handle(ctx, data, message),
        replies,
        no_ping::handle(ctx, data, message),
    );

    for (module, result) in [
        ("logging", log_result),
        ("autoupload", upload_result),
        ("chatbot", chatbot_result),
        ("checks", checks_result),
        ("message-replies", replies_result),
        ("no-ping", no_ping_result),
    ] {
        if let Err(error) = result {
            tracing::error!(
                error = %format_args!("{error:#}"),
                bot = data.bot.id,
                message_id = %message.id,
                module,
                "message handler failed"
            );
        }
    }
}
