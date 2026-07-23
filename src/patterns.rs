use std::collections::HashMap;

use anyhow::{Context as _, Result, ensure};
use regex::{Regex, RegexSet};

use crate::config::BotDefinition;

pub struct PatternService {
    paste: HashMap<&'static str, Vec<Regex>>,
    prompt_injection: HashMap<&'static str, RegexSet>,
}

impl PatternService {
    pub fn compile(bots: &[&'static BotDefinition]) -> Result<Self> {
        let mut paste = HashMap::new();
        let mut prompt_injection = HashMap::new();

        for bot in bots {
            let paste_patterns = bot
                .checks
                .paste_checks
                .iter()
                .map(|check| {
                    Regex::new(check.pattern).with_context(|| {
                        format!(
                            "{} has an invalid paste pattern `{}`",
                            bot.name, check.pattern
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let injection_patterns = RegexSet::new(bot.chatbot.ai.prompt_injection_patterns)
                .with_context(|| format!("{} has an invalid prompt-injection pattern", bot.name))?;

            ensure!(
                paste.insert(bot.id, paste_patterns).is_none()
                    && prompt_injection
                        .insert(bot.id, injection_patterns)
                        .is_none(),
                "bot ID `{}` is configured more than once",
                bot.id
            );
        }

        Ok(Self {
            paste,
            prompt_injection,
        })
    }

    pub fn is_prompt_injection(&self, bot: &BotDefinition, content: &str) -> bool {
        self.prompt_injection
            .get(bot.id)
            .is_some_and(|patterns| patterns.is_match(content))
    }

    pub fn paste_patterns(&self, bot: &BotDefinition) -> &[Regex] {
        self.paste.get(bot.id).map_or(&[], Vec::as_slice)
    }
}
