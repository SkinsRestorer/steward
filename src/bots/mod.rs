mod jarvis;
mod steward;

use crate::config::BotDefinition;

pub const ALL: &[&BotDefinition] = &[&steward::BOT, &jarvis::BOT];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use regex::Regex;

    use super::ALL;

    #[test]
    fn bot_command_definitions_fit_discord_limits() {
        for bot in ALL {
            let mut names = HashSet::new();

            for command in bot.commands.responses {
                assert!(
                    names.insert(command.name),
                    "{} defines the `{}` command more than once",
                    bot.name,
                    command.name
                );
                assert!(
                    command.name.len() <= 32,
                    "{} command name exceeds Discord's limit",
                    command.name
                );
                assert!(
                    command.description.len() <= 100,
                    "{} command description exceeds Discord's limit",
                    command.name
                );
                assert!(
                    command.fields.len() <= 25,
                    "{} command embed exceeds Discord's field limit",
                    command.name
                );
            }

            assert!(
                bot.commands.responses.len() <= 25,
                "{} help embed exceeds Discord's field limit",
                bot.name
            );
            assert!(!names.contains("help"));
            assert!(!names.contains("resolved"));
            if bot.commands.latest.is_some() {
                assert!(!names.contains("latest"));
            }

            for pattern in bot.chatbot.ai.prompt_injection_patterns {
                Regex::new(pattern).unwrap_or_else(|error| {
                    panic!(
                        "{} has an invalid prompt-injection pattern `{pattern}`: {error}",
                        bot.name
                    )
                });
            }
            for paste_check in bot.checks.paste_checks {
                Regex::new(paste_check.pattern).unwrap_or_else(|error| {
                    panic!(
                        "{} has an invalid paste pattern `{}`: {error}",
                        bot.name, paste_check.pattern
                    )
                });
            }
        }
    }
}
