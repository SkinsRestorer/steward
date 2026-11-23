mod jarvis;
mod steward;

use crate::config::BotDefinition;

pub const ALL: &[&BotDefinition] = &[&steward::BOT, &jarvis::BOT];

#[cfg(test)]
mod tests {
    use anyhow::{Context as _, Result, ensure};

    use super::ALL;
    use crate::config::{CommandField, PasteCheck, StaticCommand};

    #[test]
    fn bot_definitions_fit_discord_limits() -> Result<()> {
        for bot in ALL {
            bot.validate()
                .with_context(|| format!("{} definition is invalid", bot.name))?;
        }
        Ok(())
    }

    #[test]
    fn rejects_definitions_that_break_discord_limits() -> Result<()> {
        let base = **ALL.first().context("no bots are defined")?;

        let mut bot = base;
        bot.application_id = 0;
        ensure!(
            bot.validate().is_err(),
            "a zero application ID must be rejected"
        );

        // One character over Discord's 1,024-character field value limit.
        let value: &'static str = Box::leak("x".repeat(1_025).into_boxed_str());
        let fields: &'static [CommandField] = Box::leak(Box::new([CommandField {
            name: "Field",
            value,
        }]));
        let mut responses = base.commands.responses.to_vec();
        responses
            .first_mut()
            .context("no commands are defined")?
            .fields = fields;
        let commands: &'static [StaticCommand] = Box::leak(responses.into_boxed_slice());
        let mut bot = base;
        bot.commands.responses = commands;
        ensure!(
            bot.validate().is_err(),
            "an oversized field value must be rejected"
        );

        let paste_checks: &'static [PasteCheck] = Box::leak(Box::new([PasteCheck {
            pattern: r"https?://paste\.example/(\w+)",
            raw_url: "https://paste.example/raw",
        }]));
        let mut bot = base;
        bot.checks.paste_checks = paste_checks;
        ensure!(
            bot.validate().is_err(),
            "a raw URL without {{code}} must be rejected"
        );
        Ok(())
    }

    #[test]
    fn rejects_oversized_built_in_command_copy() -> Result<()> {
        let base = **ALL.first().context("no bots are defined")?;
        // One character over Discord's 100-character command description limit.
        let description: &'static str = Box::leak("x".repeat(101).into_boxed_str());
        // One character over Discord's 256-character embed title limit.
        let title: &'static str = Box::leak("x".repeat(257).into_boxed_str());

        let mut bot = base;
        bot.commands.help.description = description;
        ensure!(bot.validate().is_err(), "a long /help description");

        let mut bot = base;
        bot.commands.resolved.description = description;
        ensure!(bot.validate().is_err(), "a long /resolved description");

        let mut bot = base;
        bot.commands.help.embed_title = title;
        ensure!(bot.validate().is_err(), "a long help title");

        let mut bot = base;
        bot.commands.support_context.title = title;
        ensure!(bot.validate().is_err(), "a long support redirect title");
        Ok(())
    }

    #[test]
    fn rejects_chatbot_replies_that_overflow_a_message_with_the_mention() -> Result<()> {
        let base = **ALL.first().context("no bots are defined")?;

        // 1,976 characters plus the longest "<@user_id> " mention fill 2,000.
        let mut bot = base;
        bot.chatbot.max_response_length = 1_976;
        bot.validate()?;
        bot.chatbot.max_response_length = 1_977;
        ensure!(bot.validate().is_err(), "a reply over 2,000 characters");

        let message: &'static str = Box::leak("x".repeat(1_977).into_boxed_str());
        let mut bot = base;
        bot.chatbot.generation_error_message = message;
        ensure!(bot.validate().is_err(), "a long generation error message");
        Ok(())
    }

    #[test]
    fn allows_only_configured_servers_and_direct_messages() -> Result<()> {
        for bot in ALL {
            let configured = *bot.guild_ids.first().context("no server configured")?;
            ensure!(bot.allows_guild(Some(configured)));
            ensure!(bot.allows_guild(None));
            ensure!(!bot.allows_guild(Some(poise::serenity_prelude::GuildId::new(1))));
        }
        Ok(())
    }
}
