use std::time::Duration;

use anyhow::Context as _;
use poise::{CreateReply, serenity_prelude as serenity};
use serenity::Mentionable as _;

use crate::{
    ai::{ChatMessage, neutralize_prompt_tags},
    config::{BotDefinition, StaticCommand},
    events::chatbot::collect_images,
    state::{Context, Error},
};

const MAX_FORUM_POST_TAGS: usize = 5;

pub fn all(bot: &'static BotDefinition) -> Vec<poise::Command<crate::state::AppState, Error>> {
    let mut commands = bot
        .commands
        .responses
        .iter()
        .map(|spec| {
            let mut command = static_response();
            command.name = spec.name.into();
            command.qualified_name = spec.name.into();
            command.identifying_name = format!("{}::{}", bot.id, spec.name).into();
            command.description = Some(spec.description.into());
            command
        })
        .collect::<Vec<_>>();

    let mut help_command = help();
    help_command.description = Some(bot.commands.help.description.into());
    commands.push(help_command);

    if let Some(config) = bot.commands.latest {
        let mut latest_command = latest();
        latest_command.description = Some(config.description.into());
        commands.push(latest_command);
    }

    let mut resolved_command = resolved();
    resolved_command.description = Some(bot.commands.resolved.description.into());
    resolved_command.default_member_permissions = serenity::Permissions::MANAGE_THREADS;
    resolved_command.guild_only = true;
    commands.push(resolved_command);
    // AI replies cost money and post publicly, so only moderators may request them.
    let mut reply_with_ai_command = reply_with_ai();
    reply_with_ai_command.default_member_permissions = serenity::Permissions::MANAGE_MESSAGES;
    reply_with_ai_command.guild_only = true;
    commands.extend([send_help(), send_support(), reply_with_ai_command]);
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
    // Command IDs arrive after background registration; fall back to plain names until then.
    let command_ids = ctx.data().command_ids.get();
    let mut embed = serenity::CreateEmbed::new()
        .colour(bot.accent_color)
        .title(bot.commands.help.embed_title)
        .description(bot.commands.help.embed_description);

    // One field per topic keeps the list scannable instead of a flat grid.
    for group in bot.commands.help.groups {
        let lines = bot
            .commands
            .responses
            .iter()
            .filter(|command| command.group == *group)
            .map(|command| {
                let mention = command_ids
                    .and_then(|ids| ids.get(command.name))
                    .map_or_else(
                        || format!("`/{}`", command.name),
                        |id| format!("</{}:{id}>", command.name),
                    );
                format!("{mention} {}", command.description)
            })
            .collect::<Vec<_>>();
        embed = embed.field(*group, lines.join("\n"), false);
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
    let mut embed = serenity::CreateEmbed::new()
        .colour(ctx.data().bot.accent_color)
        .title(config.title)
        .description(description);
    if let Some(url) = release_page_url(config.release_url) {
        embed = embed.url(url);
    }

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Maps a GitHub API latest-release URL to the release page with the downloads.
fn release_page_url(api_url: &str) -> Option<String> {
    let repository = api_url
        .strip_prefix("https://api.github.com/repos/")?
        .strip_suffix("/releases/latest")?;
    Some(format!("https://github.com/{repository}/releases/latest"))
}

/// Marks the current support thread as resolved.
#[poise::command(slash_command)]
async fn resolved(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx.data().bot.commands.resolved;
    let tag = serenity::ForumTagId::new(config.tag_id);
    let thread = ctx
        .channel_id()
        .to_channel(ctx.http())
        .await
        .context("failed to fetch the current channel")?
        .guild()
        .filter(|channel| channel.thread_metadata.is_some());
    let Some(thread) = thread else {
        return say_ephemeral(ctx, "This command can only be used in support forum posts.").await;
    };
    if thread.applied_tags.contains(&tag) {
        return say_ephemeral(ctx, config.already_resolved_message).await;
    }
    let parent = match thread.parent_id {
        Some(parent_id) => parent_id
            .to_channel(ctx.http())
            .await
            .context("failed to fetch the parent forum")?
            .guild(),
        None => None,
    };
    if !parent.is_some_and(|parent| {
        parent
            .available_tags
            .iter()
            .any(|forum_tag| forum_tag.id == tag)
    }) {
        return say_ephemeral(ctx, "This command can only be used in support forum posts.").await;
    }
    if thread.applied_tags.len() >= MAX_FORUM_POST_TAGS {
        return say_ephemeral(
            ctx,
            "This post already has the maximum number of tags. Remove one and try again.",
        )
        .await;
    }

    {
        let _update_guard = ctx.data().services.thread_updates.lock().await;
        let mut tags = thread.applied_tags;
        tags.push(tag);
        thread
            .id
            .edit_thread(
                ctx.http(),
                // An auto-archived post rejects tag edits and the reply below,
                // so unarchive it in the same edit.
                serenity::EditThread::new()
                    .applied_tags(tags)
                    .archived(false)
                    .audit_log_reason("resolved"),
            )
            .await
            .context("failed to tag the thread as resolved")?;
    }
    // Reply before archiving: Discord rejects interaction replies in archived threads.
    ctx.say(config.success_message).await?;
    thread
        .id
        .edit_thread(
            ctx.http(),
            serenity::EditThread::new()
                .locked(true)
                .archived(true)
                .audit_log_reason("resolved"),
        )
        .await
        .context("failed to lock and archive the resolved thread")?;
    Ok(())
}

async fn say_ephemeral(ctx: Context<'_>, content: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().ephemeral(true).content(content))
        .await?;
    Ok(())
}

/// Sends one of the bot's support responses to a selected message.
#[poise::command(
    context_menu_command = "Send Help",
    guild_only,
    required_permissions = "SEND_MESSAGES"
)]
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
    .placeholder("Select a help topic");
    let handle = ctx
        .send(
            CreateReply::default()
                .ephemeral(true)
                .content("Select a help topic.")
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
        return finish_menu(ctx, &handle, "Timed out").await;
    };
    // Acknowledge first so Discord never shows "This interaction failed".
    interaction
        .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    let command = if let serenity::ComponentInteractionDataKind::StringSelect { values } =
        &interaction.data.kind
    {
        values
            .first()
            .and_then(|name| find_static_command(bot, name).ok())
    } else {
        None
    };
    let Some(command) = command else {
        return finish_menu(ctx, &handle, "That help topic is no longer available.").await;
    };

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
        Ok(_) => finish_menu(ctx, &handle, "Help message sent.").await,
        Err(error) => {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                channel_id = %target.channel_id,
                "failed to send help message"
            );
            finish_menu(
                ctx,
                &handle,
                "The help message failed to send. Make sure that I can send messages in this channel.",
            )
            .await
        }
    }
}

async fn finish_menu(
    ctx: Context<'_>,
    handle: &poise::ReplyHandle<'_>,
    content: &str,
) -> Result<(), Error> {
    handle
        .edit(
            ctx,
            CreateReply::default()
                .content(content)
                .components(Vec::new()),
        )
        .await?;
    Ok(())
}

/// Redirects a message to the support forum.
#[poise::command(
    context_menu_command = "Send to Support Forum",
    guild_only,
    required_permissions = "SEND_MESSAGES"
)]
async fn send_support(ctx: Context<'_>, target: serenity::Message) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
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
        .await
        .context("failed to send the support redirect")?;
    ctx.say("Support redirect sent.").await?;
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
    let images = collect_images(&target.attachments);
    if prompt.is_empty() && images.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("The selected message has no text or supported images to reply to."),
        )
        .await?;
        return Ok(());
    }

    if ctx
        .data()
        .services
        .patterns
        .is_prompt_injection(ctx.data().bot, prompt)
    {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content(ctx.data().bot.chatbot.prompt_injection_error_message),
        )
        .await?;
        return Ok(());
    }

    ctx.defer_ephemeral().await?;
    let requester_prefix = format!(
        "{} requested me to reply to this message.",
        ctx.author().mention()
    );
    // Add the requester line in code. Asking the model to write it would put an
    // instruction inside the untrusted user-message block.
    let max_length = ctx
        .data()
        .bot
        .chatbot
        .max_response_length
        .saturating_sub(requester_prefix.chars().count().saturating_add(1));

    match ctx
        .data()
        .services
        .ai
        .generate_response(
            &[ChatMessage::User(
                neutralize_prompt_tags(prompt).into(),
                images,
            )],
            None,
            ctx.data().bot.chatbot.ai,
            max_length,
            None,
        )
        .await
    {
        Ok(response) => {
            let response = format!("{requester_prefix} {response}");
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
                        // The paid reply must still post if the target was deleted meanwhile.
                        .reference_message(
                            serenity::MessageReference::from(&target).fail_if_not_exists(false),
                        )
                        .allowed_mentions(
                            serenity::CreateAllowedMentions::new()
                                .users(allowed_users)
                                .replied_user(true),
                        ),
                )
                .await
                .context("failed to send the AI reply")?;
            ctx.say("Reply sent.").await?;
        }
        Err(error) => {
            tracing::error!(error = %format_args!("{error:#}"), "failed to generate context-menu AI reply");
            ctx.say("Reply generation failed. Try again later.").await?;
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
    // The linked title is the call to action, so the URL is not repeated below.
    let mut embed = serenity::CreateEmbed::new()
        .colour(bot.accent_color)
        .title(command.title)
        .description(command.body);
    if let Some(url) = command.url {
        embed = embed.url(url);
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

#[cfg(test)]
mod release_page_tests {
    use super::release_page_url;

    #[test]
    fn links_github_api_release_urls_to_the_release_page() {
        assert_eq!(
            release_page_url(
                "https://api.github.com/repos/SkinsRestorer/SkinsRestorer/releases/latest"
            )
            .as_deref(),
            Some("https://github.com/SkinsRestorer/SkinsRestorer/releases/latest")
        );
    }

    #[test]
    fn skips_urls_that_are_not_github_latest_release_endpoints() {
        assert_eq!(
            release_page_url("https://example.com/releases/latest"),
            None
        );
        assert_eq!(
            release_page_url("https://api.github.com/repos/SkinsRestorer/SkinsRestorer/releases"),
            None
        );
    }
}

#[cfg(test)]
mod registration_tests {
    use super::all;
    use crate::bots;
    use poise::serenity_prelude as serenity;

    #[test]
    fn registers_bot_specific_commands_and_moderator_permissions() {
        for bot in bots::ALL {
            let commands = all(bot);
            assert_eq!(
                commands.len(),
                bot.commands.responses.len() + 5 + usize::from(bot.commands.latest.is_some())
            );
            for spec in bot.commands.responses {
                let command = commands
                    .iter()
                    .find(|command| command.name == spec.name)
                    .expect("configured response command");
                assert_eq!(command.qualified_name, spec.name);
                assert_eq!(
                    command.identifying_name,
                    format!("{}::{}", bot.id, spec.name)
                );
                assert_eq!(command.description.as_deref(), Some(spec.description));
                assert!(command.slash_action.is_some());
            }
            let resolved = commands
                .iter()
                .find(|command| command.name == "resolved")
                .expect("moderator command");
            assert!(resolved.guild_only);
            assert_eq!(
                resolved.default_member_permissions,
                serenity::Permissions::MANAGE_THREADS
            );
            assert_eq!(
                resolved.description.as_deref(),
                Some(bot.commands.resolved.description)
            );
            let reply_with_ai = commands
                .iter()
                .find(|command| command.context_menu_name.as_deref() == Some("Reply with AI"))
                .expect("AI context menu command");
            assert!(reply_with_ai.guild_only);
            assert_eq!(
                reply_with_ai.default_member_permissions,
                serenity::Permissions::MANAGE_MESSAGES
            );
            let latest = commands.iter().find(|command| command.name == "latest");
            assert_eq!(latest.is_some(), bot.commands.latest.is_some());
        }
    }
}
