mod jarvis;
mod steward;

use crate::config::BotDefinition;

pub const ALL: &[&BotDefinition] = &[&steward::BOT, &jarvis::BOT];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use anyhow::{Context as _, Result, ensure};
    use regex::Regex;

    use super::ALL;

    #[test]
    fn bot_command_definitions_fit_discord_limits() -> Result<()> {
        for bot in ALL {
            let mut names = HashSet::new();

            for command in bot.commands.responses {
                ensure!(
                    names.insert(command.name),
                    "{} defines the `{}` command more than once",
                    bot.name,
                    command.name
                );
                ensure!(
                    command.name.len() <= 32,
                    "{} command name exceeds Discord's limit",
                    command.name
                );
                ensure!(
                    command.description.len() <= 100,
                    "{} command description exceeds Discord's limit",
                    command.name
                );
                ensure!(
                    command.fields.len() <= 25,
                    "{} command embed exceeds Discord's field limit",
                    command.name
                );
            }

            ensure!(
                bot.commands.responses.len() <= 25,
                "{} help embed exceeds Discord's field limit",
                bot.name
            );
            ensure!(
                !names.contains("help"),
                "{} defines a reserved command",
                bot.name
            );
            ensure!(
                !names.contains("resolved"),
                "{} defines a reserved command",
                bot.name
            );
            if bot.commands.latest.is_some() {
                ensure!(
                    !names.contains("latest"),
                    "{} defines a reserved command",
                    bot.name
                );
            }

            for pattern in bot.chatbot.ai.prompt_injection_patterns {
                Regex::new(pattern).with_context(|| {
                    format!(
                        "{} has an invalid prompt-injection pattern `{pattern}`",
                        bot.name
                    )
                })?;
            }
            for paste_check in bot.checks.paste_checks {
                Regex::new(paste_check.pattern).with_context(|| {
                    format!(
                        "{} has an invalid paste pattern `{}`",
                        bot.name, paste_check.pattern
                    )
                })?;
            }
        }
        Ok(())
    }
}
