use std::{env, sync::Arc};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::{
    commands,
    config::BotDefinition,
    events,
    state::{AppState, Error, SharedServices},
};

pub async fn start_bot(bot: &'static BotDefinition, services: Arc<SharedServices>) -> Result<()> {
    let token = env::var(bot.token_env)
        .with_context(|| format!("{} must be configured for {}", bot.token_env, bot.name))?;
    let state = AppState::new(bot, services);
    let setup_state = state.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(bot),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(events::handle(ctx, event, data))
            },
            on_error: |error| Box::pin(handle_framework_error(error)),
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            let state = setup_state.clone();
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let registered = serenity::Command::get_global_commands(&ctx.http).await?;
                let mut command_ids = state.command_ids.write().await;
                command_ids.extend(
                    registered
                        .into_iter()
                        .map(|command| (command.name, command.id)),
                );
                drop(command_ids);

                tracing::info!(
                    bot = bot.id,
                    user = %ready.user.tag(),
                    commands = framework.options().commands.len(),
                    "registered global application commands"
                );
                Ok(state)
            })
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

async fn handle_framework_error(error: poise::FrameworkError<'_, AppState, Error>) {
    tracing::error!(%error, "Discord framework error");
    if let Err(reporting_error) = poise::builtins::on_error(error).await {
        tracing::error!(%reporting_error, "failed to report Discord framework error");
    }
}
