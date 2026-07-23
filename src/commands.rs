use std::time::Duration;

use anyhow::{Context as _, bail};
use poise::{CreateReply, serenity_prelude as serenity};
use serenity::Mentionable as _;

use crate::{
    ai::ChatMessage,
    config::{BotDefinition, StaticCommand},
    state::{Context, Error},
};

pub fn all(bot: &'static BotDefinition) -> Vec<poise::Command<crate::state::AppState, Error>> {
    let mut commands = bot
        .commands
        .responses
        .iter()
        .map(|spec| {
            let mut command = static_response();
            spec.name.clone_into(&mut command.name);
            spec.name.clone_into(&mut command.qualified_name);
            command.identifying_name = format!("{}::{}", bot.id, spec.name);
            command.description = Some(spec.description.to_owned());
            command
        })
        .collect::<Vec<_>>();

    let mut help_command = help();
    help_command.description = Some(bot.commands.help.description.to_owned());
    commands.push(help_command);

    if let Some(config) = bot.commands.latest {
        let mut latest_command = latest();
        latest_command.description = Some(config.description.to_owned());
        commands.push(latest_command);
    }

    let mut resolved_command = resolved();
    resolved_command.description = Some(bot.commands.resolved.description.to_owned());
    resolved_command.default_member_permissions = serenity::Permissions::MANAGE_THREADS;
    resolved_command.guild_only = true;
    commands.push(resolved_command);
    commands.extend([send_help(), send_support(), reply_with_ai()]);
    commands
}

/// Sends a configured support response.
#[poise::command(slash_command)]
async fn static_response(
    ctx: Context<'_>,
    #[description = "Mention a specific user with the command"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let command = find_static_command(ctx.data().bot, &ctx.command().name)?;
    let reply = CreateReply::default()
        .content(
            user.map(|user| user.mention().to_string())
                .unwrap_or_default(),
        )
        .embed(static_response_embed(ctx.data().bot, command));
    ctx.send(reply).await?;
    Ok(())
}

/// Shows the bot's available support commands.
#[poise::command(slash_command)]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let bot = ctx.data().bot;
    let command_ids = ctx.data().command_ids.read().await;
    let mut embed = serenity::CreateEmbed::new()
        .colour(bot.accent_color)
        .title(bot.commands.help.embed_title)
        .description(bot.commands.help.embed_description);

    for command in bot.commands.responses {
        let id = command_ids
            .get(command.name)
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);
        embed = embed.field(
            format!("</{}:{id}>", command.name),
            command.description,
            true,
        );
    }

    ctx.send(CreateReply::default().ephemeral(true).embed(embed))
        .await?;
    Ok(())
}

/// Shows the latest released version.
#[poise::command(slash_command)]
async fn latest(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx
        .data()
        .bot
        .commands
        .latest
        .context("latest command is not configured")?;
    let release = ctx.data().services.releases.get(config.release_url).await;
    let description = release.map_or_else(
        || "The latest version is not available yet. Try again in a moment.".to_owned(),
        |release| format!("`{}`", release.tag_name),
    );
    let embed = serenity::CreateEmbed::new()
        .colour(ctx.data().bot.accent_color)
        .title(config.title)
        .description(description);

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Marks the current support thread as resolved.
#[poise::command(slash_command)]
async fn resolved(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx.data().bot.commands.resolved;
    let channel = ctx.channel_id().to_channel(ctx.http()).await?;
    let serenity::Channel::Guild(thread) = channel else {
        ctx.say("This command can only be used in threads.").await?;
        return Ok(());
    };
    if !matches!(
        thread.kind,
        serenity::ChannelType::PublicThread
            | serenity::ChannelType::PrivateThread
            | serenity::ChannelType::NewsThread
    ) {
        ctx.say("This command can only be used in threads.").await?;
        return Ok(());
    }

    let tag = serenity::ForumTagId::new(config.tag_id);
    if thread.applied_tags.contains(&tag) {
        ctx.say(config.already_resolved_message).await?;
        return Ok(());
    }

    ctx.say(config.success_message).await?;
    let mut tags = thread.applied_tags;
    tags.push(tag);
    thread
        .id
        .edit_thread(
            ctx.http(),
            serenity::EditThread::new()
                .applied_tags(tags)
                .locked(true)
                .archived(true)
                .audit_log_reason("resolved"),
        )
        .await?;
    Ok(())
}

/// Sends one of the bot's support responses to a selected message.
#[poise::command(context_menu_command = "Send Help")]
async fn send_help(ctx: Context<'_>, target: serenity::Message) -> Result<(), Error> {
    let bot = ctx.data().bot;
    let custom_id = format!("help-selection-{}-{}", bot.id, ctx.id());
    let options = bot
        .commands
        .responses
        .iter()
        .map(|command| {
            serenity::CreateSelectMenuOption::new(command.name, command.name)
                .description(command.description)
        })
        .collect();
    let menu = serenity::CreateSelectMenu::new(
        &custom_id,
        serenity::CreateSelectMenuKind::String { options },
    )
    .placeholder("Choose a help type!");
    let handle = ctx
        .send(
            CreateReply::default()
                .ephemeral(true)
                .content("Please select a help type!")
                .components(vec![serenity::CreateActionRow::SelectMenu(menu)]),
        )
        .await?;

    let interaction = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(Duration::from_mins(1))
        .filter(move |interaction| interaction.data.custom_id == custom_id)
        .await;

    let Some(interaction) = interaction else {
        handle
            .edit(
                ctx,
                CreateReply::default()
                    .content("Timed out")
                    .components(Vec::new()),
            )
            .await?;
        return Ok(());
    };
    let serenity::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind
    else {
        bail!("help selection returned an unexpected component type");
    };
    let command_name = values.first().context("help selection had no value")?;
    let command = find_static_command(bot, command_name)?;

    interaction
        .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    let result = target
        .channel_id
        .send_message(
            ctx.http(),
            static_response_message(bot, command)
                .content(format!("Requested by {}", ctx.author().mention()))
                .reference_message(&target),
        )
        .await;

    match result {
        Ok(_) => {
            handle
                .edit(
                    ctx,
                    CreateReply::default()
                        .content("Help message sent.")
                        .components(Vec::new()),
                )
                .await?;
        }
        Err(error) => {
            handle
                .edit(
                    ctx,
                    CreateReply::default()
                        .content(
                            "Failed to send the help message. Please check my permissions and try again.",
                        )
                        .components(Vec::new()),
                )
                .await?;
            return Err(error.into());
        }
    }

    Ok(())
}

/// Redirects a message to the support forum.
#[poise::command(context_menu_command = "Send to Support Forum")]
async fn send_support(ctx: Context<'_>, target: serenity::Message) -> Result<(), Error> {
    let bot = ctx.data().bot;
    let config = bot.commands.support_context;
    let mut embed = serenity::CreateEmbed::new()
        .colour(bot.accent_color)
        .title(config.title)
        .description(config.description);
    if let Some(url) = config.url {
        embed = embed.url(url);
    }

    target
        .channel_id
        .send_message(
            ctx.http(),
            serenity::CreateMessage::new()
                .content(format!(
                    "{} Requested by {}",
                    target.author.mention(),
                    ctx.author().mention()
                ))
                .embed(embed)
                .reference_message(&target),
        )
        .await?;
    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .content("Support redirect sent."),
    )
    .await?;
    Ok(())
}

/// Generates a support response for a selected message.
#[poise::command(context_menu_command = "Reply with AI")]
async fn reply_with_ai(ctx: Context<'_>, target: serenity::Message) -> Result<(), Error> {
    if target.author.bot {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("I cannot reply to bot messages."),
        )
        .await?;
        return Ok(());
    }

    let prompt = target.content.trim();
    if prompt.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("The selected message has no text to reply to."),
        )
        .await?;
        return Ok(());
    }

    ctx.defer_ephemeral().await?;
    let requester_prefix = format!(
        "{} requested me to reply to this message.",
        ctx.author().mention()
    );
    let request = format!(
        "Begin your response with \"{requester_prefix}\" exactly before offering help. Here is the message you must answer:\n{prompt}"
    );

    match ctx
        .data()
        .services
        .ai
        .generate_response(
            &[ChatMessage::User(request)],
            ctx.data().bot.chatbot.ai,
            1_300,
        )
        .await
    {
        Ok(response) => {
            let response = if response.starts_with(&requester_prefix) {
                response
            } else {
                format!("{requester_prefix} {response}")
            };
            let allowed_users = if target.author.id == ctx.author().id {
                vec![ctx.author().id]
            } else {
                vec![target.author.id, ctx.author().id]
            };
            target
                .channel_id
                .send_message(
                    ctx.http(),
                    serenity::CreateMessage::new()
                        .content(response)
                        .reference_message(&target)
                        .allowed_mentions(
                            serenity::CreateAllowedMentions::new()
                                .users(allowed_users)
                                .replied_user(true),
                        ),
                )
                .await?;
            ctx.say("Reply sent.").await?;
        }
        Err(error) => {
            tracing::error!(%error, "failed to generate context-menu AI reply");
            ctx.say("Failed to generate a reply. Please try again later.")
                .await?;
        }
    }

    Ok(())
}

fn find_static_command(
    bot: &'static BotDefinition,
    name: &str,
) -> Result<&'static StaticCommand, Error> {
    bot.commands
        .responses
        .iter()
        .find(|command| command.name == name)
        .with_context(|| format!("unknown configured command {name}"))
}

fn static_response_embed(
    bot: &'static BotDefinition,
    command: &StaticCommand,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .colour(bot.accent_color)
        .description(command.body);

    if command.documentation {
        embed = embed.title(format!("🔖 {}", command.title));
        let mut footer = serenity::CreateEmbedFooter::new(bot.commands.docs_footer.text);
        if let Some(icon_url) = bot.commands.docs_footer.icon_url {
            footer = footer.icon_url(icon_url);
        }
        embed = embed.footer(footer);
        if let Some(url) = command.url {
            embed = embed.field("Read more", url, false).url(url);
        }
    } else {
        embed = embed.title(command.title);
        if let Some(url) = command.url {
            embed = embed.field("Link", url, false).url(url);
        }
    }

    for field in command.fields {
        embed = embed.field(field.name, field.value, false);
    }
    embed
}

fn static_response_message(
    bot: &'static BotDefinition,
    command: &StaticCommand,
) -> serenity::CreateMessage {
    serenity::CreateMessage::new().embed(static_response_embed(bot, command))
}
