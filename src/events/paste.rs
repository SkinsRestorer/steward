use std::collections::HashSet;

use poise::serenity_prelude as serenity;
use regex::Regex;

use crate::{config::PasteCheck, state::AppState};

#[derive(Clone, Debug)]
pub(super) struct PasteLink {
    pub original_url: String,
    pub raw_url: String,
}

pub(super) fn find_all(message: &serenity::Message, data: &AppState) -> Vec<PasteLink> {
    let mut contents = vec![message.content.as_str()];
    contents.extend(
        message
            .embeds
            .iter()
            .flat_map(|embed| embed.fields.iter().map(|field| field.value.as_str())),
    );
    find_in_contents(
        &contents,
        data.bot.checks.paste_checks,
        data.services.patterns.paste_patterns(data.bot),
    )
}

fn find_in_contents(
    contents: &[&str],
    checks: &[PasteCheck],
    patterns: &[Regex],
) -> Vec<PasteLink> {
    let mut raw_urls = HashSet::new();
    let mut matches = Vec::new();

    for (check, regex) in checks.iter().zip(patterns) {
        for content in contents {
            for captures in regex.captures_iter(content) {
                let (Some(code), Some(original_url)) = (captures.get(1), captures.get(0)) else {
                    continue;
                };
                let raw_url = check.raw_url.replace("{code}", code.as_str());
                if raw_urls.insert(raw_url.clone()) {
                    matches.push(PasteLink {
                        raw_url,
                        original_url: original_url.as_str().to_owned(),
                    });
                }
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use anyhow::{Context as _, Result, ensure};
    use regex::Regex;

    use super::find_in_contents;
    use crate::config::PasteCheck;

    #[test]
    fn finds_all_unique_allowed_paste_links() -> Result<()> {
        const CHECKS: &[PasteCheck] = &[
            PasteCheck {
                pattern: r"https?://pastes\.example/(\w+)",
                raw_url: "https://raw.example/{code}",
            },
            PasteCheck {
                pattern: r"https?://snippets\.example/(\w+)",
                raw_url: "https://snippets.example/{code}/raw",
            },
        ];
        let patterns = CHECKS
            .iter()
            .map(|check| Regex::new(check.pattern))
            .collect::<Result<Vec<_>, _>>()
            .context("test paste patterns should compile")?;

        let links = find_in_contents(
            &[
                "See https://pastes.example/alpha and https://snippets.example/beta",
                "Duplicate: https://pastes.example/alpha",
            ],
            CHECKS,
            &patterns,
        );

        ensure!(links.len() == 2);
        ensure!(links[0].raw_url == "https://raw.example/alpha");
        ensure!(links[1].raw_url == "https://snippets.example/beta/raw");
        Ok(())
    }
}
