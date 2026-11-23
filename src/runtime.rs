use std::{
    collections::HashMap,
    env,
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::{Context as _, Result, ensure};
use poise::serenity_prelude as serenity;

use crate::{
    commands,
    config::BotDefinition,
    events,
    state::{AppState, Error, SharedServices},
};

const COMMAND_REGISTRATION_INITIAL_DELAY: Duration = Duration::from_secs(5);
const COMMAND_REGISTRATION_MAX_DELAY: Duration = Duration::from_mins(5);

pub async fn start_bot(bot: &'static BotDefinition, services: Arc<SharedServices>) -> Result<()> {
    let token = env::var(bot.token_env)
        .with_context(|| format!("{} must be configured for {}", bot.token_env, bot.name))?;
    ensure!(
        !token.trim().is_empty(),
        "{} must be configured for {}",
        bot.token_env,
        bot.name
    );
    let state = AppState::new(bot, services);
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(bot),
            event_handler: |framework, event| {
                Box::pin(events::handle(
                    framework.serenity_context,
                    event,
                    framework.user_data,
                ))
            },
            on_error: |error| Box::pin(handle_framework_error(error)),
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            // poise runs setup only once and never retries it. Keep it infallible and
            // register commands in the background so a Discord error at startup
            // cannot leave the bot connected but unable to handle events.
            let commands =
                poise::builtins::create_application_commands(&framework.options().commands);
            tokio::spawn(register_commands(
                Arc::clone(&ctx.http),
                bot,
                commands,
                Arc::clone(&state.command_ids),
            ));
            tracing::info!(bot = bot.id, user = %ready.user.tag(), "Discord bot ready");
            Box::pin(async move { Ok(state) })
        })
        .build();

    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_PRESENCES
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
        | serenity::GatewayIntents::GUILD_MESSAGE_TYPING
        | serenity::GatewayIntents::MESSAGE_CONTENT;
    let mut client = serenity::ClientBuilder::new(token, intents)
        .application_id(serenity::ApplicationId::new(bot.application_id))
        .activity(serenity::ActivityData::watching(bot.presence))
        .status(serenity::OnlineStatus::Online)
        .framework(framework)
        .await
        .with_context(|| format!("failed to create {} Discord client", bot.name))?;

    client
        .start()
        .await
        .with_context(|| format!("{} Discord client stopped with an error", bot.name))
}

async fn register_commands(
    http: Arc<serenity::Http>,
    bot: &'static BotDefinition,
    commands: Vec<serenity::CreateCommand>,
    command_ids: Arc<OnceLock<HashMap<String, serenity::CommandId>>>,
) {
    let mut delay = COMMAND_REGISTRATION_INITIAL_DELAY;
    loop {
        match serenity::Command::set_global_commands(&http, commands.clone()).await {
            Ok(registered) => {
                let count = registered.len();
                let ids = registered
                    .into_iter()
                    .map(|command| (command.name, command.id))
                    .collect();
                if command_ids.set(ids).is_err() {
                    tracing::warn!(bot = bot.id, "application command IDs were already set");
                }
                tracing::info!(
                    bot = bot.id,
                    commands = count,
                    "registered global application commands"
                );
                return;
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    bot = bot.id,
                    retry_in_secs = delay.as_secs(),
                    "failed to register global application commands"
                );
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2).min(COMMAND_REGISTRATION_MAX_DELAY);
            }
        }
    }
}

async fn handle_framework_error(error: poise::FrameworkError<'_, AppState, Error>) {
    // poise's default handler prints command errors with eprintln and shows the
    // raw error chain to the user. Log them through tracing and reply generically.
    if let poise::FrameworkError::Command { error, ctx, .. } = error {
        tracing::error!(
            error = %format_args!("{error:#}"),
            command = %ctx.command().qualified_name,
            guild_id = ?ctx.guild_id(),
            channel_id = %ctx.channel_id(),
            user_id = %ctx.author().id,
            "Discord command failed"
        );
        let reply = poise::CreateReply::default()
            .ephemeral(true)
            .content("Something went wrong. Try again in a moment.");
        if let Err(reporting_error) = ctx.send(reply).await {
            tracing::warn!(
                error = %format_args!("{reporting_error:#}"),
                "failed to report a command error"
            );
        }
        return;
    }

    tracing::error!(%error, "Discord framework error");
    if let Err(reporting_error) = poise::builtins::on_error(error).await {
        tracing::error!(%reporting_error, "failed to report Discord framework error");
    }
}
