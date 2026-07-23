mod autoupload;
pub mod chatbot;
mod checks;
mod logging;
mod message_replies;
mod no_ping;
mod thread_starter;

use poise::serenity_prelude as serenity;

use crate::state::{AppState, Error};

pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &AppState,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            tracing::info!(
                bot = data.bot.id,
                user = %data_about_bot.user.tag(),
                "Discord bot connected"
            );
        }
        serenity::FullEvent::Message { new_message } => {
            handle_message(ctx, data, new_message).await;
        }
        serenity::FullEvent::ThreadCreate { thread } => {
            if let Err(error) = thread_starter::handle(ctx, data, thread).await {
                tracing::error!(%error, bot = data.bot.id, "thread starter handler failed");
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_message(ctx: &serenity::Context, data: &AppState, message: &serenity::Message) {
    if message.author.bot || message.guild_id.is_none() {
        return;
    }

    let channel_name = message.guild_id.and_then(|guild_id| {
        ctx.cache.guild(guild_id).and_then(|guild| {
            guild
                .channels
                .get(&message.channel_id)
                .map(|channel| channel.name.clone())
                .or_else(|| {
                    guild
                        .threads
                        .iter()
                        .find(|thread| thread.id == message.channel_id)
                        .map(|thread| thread.name.clone())
                })
        })
    });

    let (log_result, upload_result, chatbot_result, checks_result, replies_result, no_ping_result) = tokio::join!(
        logging::handle(data, message, channel_name.as_deref()),
        autoupload::handle(ctx, data, message),
        chatbot::handle(ctx, data, message, channel_name.as_deref()),
        checks::handle(ctx, data, message),
        message_replies::handle(ctx, data, message),
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
                %error,
                bot = data.bot.id,
                message_id = %message.id,
                module,
                "message handler failed"
            );
        }
    }
}
