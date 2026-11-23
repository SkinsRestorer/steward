use std::collections::HashSet;

use anyhow::{Context as _, Result, ensure};
use poise::serenity_prelude as serenity;
use regex::Regex;

use crate::releases::LatestRelease;

#[derive(Clone, Copy, Debug)]
pub struct CommandField {
    pub name: &'static str,
    pub value: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct StaticCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub url: Option<&'static str>,
    pub documentation: bool,
    pub fields: &'static [CommandField],
}

#[derive(Clone, Copy, Debug)]
pub struct AiConfig {
    pub application_guardrail: &'static str,
    pub docs_context_urls: &'static [&'static str],
    /// When set, the model can read single documentation pages whose URL
    /// starts with this prefix, for example pages listed in an llms.txt index.
    pub docs_page_prefix: Option<&'static str>,
    /// Links the model may post. Each entry is a host, which also covers its
    /// subdomains, or a host and path prefix such as `github.com/SkinsRestorer`.
    /// Other links are removed from AI replies.
    pub allowed_links: &'static [&'static str],
    pub model: &'static str,
    pub prompt_injection_patterns: &'static [&'static str],
    pub response_disclaimer: &'static str,
    pub system_prompt: &'static str,
    pub web_search_max_tokens: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct ChatbotConfig {
    pub ai: &'static AiConfig,
    pub channel_name_prefixes: &'static [&'static str],
    pub generation_error_message: &'static str,
    pub max_response_length: usize,
    pub prompt_injection_error_message: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct PasteCheck {
    pub pattern: &'static str,
    pub raw_url: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct TextCheck {
    pub needle: &'static str,
    pub title: &'static str,
    pub content: &'static str,
    pub tips: &'static [&'static str],
    pub link: Option<&'static str>,
}

/// Returns summary embeds for a product's debug dump, or `None` when the text is
/// not such a dump.
pub type DumpAnalyzer = fn(&str, Option<&LatestRelease>) -> Option<Vec<serenity::CreateEmbed>>;

#[derive(Clone, Copy, Debug)]
pub struct ChecksConfig {
    pub paste_checks: &'static [PasteCheck],
    pub release_url: Option<&'static str>,
    pub text_checks: &'static [TextCheck],
    /// Turns a pasted debug dump into summary embeds. `None` disables dump analysis.
    pub dump_analyzer: Option<DumpAnalyzer>,
}

#[derive(Clone, Copy, Debug)]
pub struct AutouploadConfig {
    pub user_agent: &'static str,
    pub future_uploads_message: fn(&str, &str) -> String,
    pub failed_upload_message: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct DocsFooter {
    pub text: &'static str,
    pub icon_url: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct HelpConfig {
    pub description: &'static str,
    pub embed_title: &'static str,
    pub embed_description: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct LatestConfig {
    pub description: &'static str,
    pub release_url: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedConfig {
    pub already_resolved_message: &'static str,
    pub description: &'static str,
    pub success_message: &'static str,
    pub tag_id: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct SupportContextConfig {
    pub description: &'static str,
    pub title: &'static str,
    pub url: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct CommandsConfig {
    pub responses: &'static [StaticCommand],
    pub docs_footer: DocsFooter,
    pub help: HelpConfig,
    pub latest: Option<LatestConfig>,
    pub resolved: ResolvedConfig,
    pub support_context: SupportContextConfig,
}

#[derive(Clone, Copy, Debug)]
pub struct NoPingConfig {
    pub exempt_role_ids: &'static [u64],
    pub staff_role_ids: &'static [u64],
    pub warning_message: fn(serenity::UserId) -> String,
}

#[derive(Clone, Copy, Debug)]
pub struct HoneypotConfig {
    pub channel_names: &'static [&'static str],
    pub delete_message_days: u8,
    pub exempt_role_ids: &'static [u64],
}

#[derive(Clone, Copy, Debug)]
pub struct ThreadStarterConfig {
    /// Forums whose new posts get the support and priority-support message.
    pub forum_ids: &'static [serenity::ChannelId],
    pub support_title: &'static str,
    pub support_description: &'static str,
    pub docs_url: &'static str,
    pub priority_title: &'static str,
    pub priority_description: &'static str,
    pub priority_banner_url: &'static str,
    pub pricing_url: &'static str,
    pub priority_support_url: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub enum ReplyTrigger {
    /// The message, lowercased and reduced to ASCII letters, digits, and `_`,
    /// contains this text.
    NormalizedContains(&'static str),
    /// The message starts with this server command, case-insensitively, and
    /// has at most one argument. People sometimes type server commands in
    /// Discord by mistake.
    ServerCommand(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct MessageReply {
    pub trigger: ReplyTrigger,
    pub title: &'static str,
    pub description: &'static str,
    pub thumbnail_url: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct BotDefinition {
    pub id: &'static str,
    /// Servers where the bot works. Anyone can invite a public bot, so the bot
    /// ignores messages, threads, and commands from every other server.
    pub guild_ids: &'static [serenity::GuildId],
    pub name: &'static str,
    pub token_env: &'static str,
    pub application_id: u64,
    pub accent_color: u32,
    pub logs_dir: &'static str,
    pub presence: &'static str,
    // Each optional feature is off when its config is `None`.
    pub autoupload: Option<AutouploadConfig>,
    pub chatbot: ChatbotConfig,
    pub checks: ChecksConfig,
    pub commands: CommandsConfig,
    pub no_ping: Option<NoPingConfig>,
    pub honeypot: Option<HoneypotConfig>,
    pub thread_starter: Option<ThreadStarterConfig>,
    /// Canned replies to common mistakes in messages.
    pub message_replies: &'static [MessageReply],
}

// Discord limits that static bot copy must respect.
const MAX_COMMAND_NAME_CHARS: usize = 32;
const MAX_COMMAND_DESCRIPTION_CHARS: usize = 100;
const MAX_EMBED_TITLE_CHARS: usize = 256;
const MAX_EMBED_DESCRIPTION_CHARS: usize = 4_096;
const MAX_EMBED_FIELDS: usize = 25;
const MAX_EMBED_FIELD_NAME_CHARS: usize = 256;
const MAX_EMBED_FIELD_VALUE_CHARS: usize = 1_024;
const MAX_SELECT_OPTIONS: usize = 25;
const MAX_BAN_MESSAGE_DAYS: u8 = 7;
const RESERVED_COMMAND_NAMES: &[&str] = &["help", "latest", "resolved"];

impl BotDefinition {
    /// Whether the bot may act in this server. Direct messages have no server.
    pub fn allows_guild(&self, guild_id: Option<serenity::GuildId>) -> bool {
        guild_id.is_none_or(|guild_id| self.guild_ids.contains(&guild_id))
    }

    /// Checks the static definition against Discord's limits, so a bad edit
    /// fails at startup instead of at command registration or the first reply.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.application_id != 0, "application_id must not be 0");
        ensure!(
            !self.guild_ids.is_empty(),
            "guild_ids must list at least one server"
        );
        ensure!(
            !self.chatbot.ai.system_prompt.trim().is_empty(),
            "the AI system prompt must not be empty"
        );
        ensure!(
            self.commands.resolved.tag_id != 0,
            "the resolved tag ID must not be 0"
        );
        if let Some(honeypot) = self.honeypot {
            ensure!(
                honeypot.delete_message_days <= MAX_BAN_MESSAGE_DAYS,
                "honeypot cleanup exceeds Discord's seven-day limit"
            );
        }

        self.validate_commands()?;
        self.validate_copy()?;
        self.validate_docs_page_prefix()?;
        for entry in self.chatbot.ai.allowed_links {
            ensure!(
                !entry.contains("://") && reqwest::Url::parse(&format!("https://{entry}")).is_ok(),
                "allowed link `{entry}` must be a host or host/path without a scheme"
            );
        }
        self.validate_paste_checks()
    }

    fn validate_commands(&self) -> Result<()> {
        let mut names = HashSet::new();
        for command in self.commands.responses {
            ensure!(
                names.insert(command.name),
                "the `{}` command is defined more than once",
                command.name
            );
            ensure!(
                !RESERVED_COMMAND_NAMES.contains(&command.name),
                "`{}` is a reserved command name",
                command.name
            );
            check_length("command name", command.name, MAX_COMMAND_NAME_CHARS)?;
            check_length(
                "command description",
                command.description,
                MAX_COMMAND_DESCRIPTION_CHARS,
            )?;
            // The documentation style prefixes the title with an emoji and a space.
            check_length(
                "command title",
                command.title,
                MAX_EMBED_TITLE_CHARS.saturating_sub(2),
            )?;
            check_length("command body", command.body, MAX_EMBED_DESCRIPTION_CHARS)?;
            // static_response_embed adds a link field when the command has a URL.
            ensure!(
                command
                    .fields
                    .len()
                    .saturating_add(usize::from(command.url.is_some()))
                    <= MAX_EMBED_FIELDS,
                "the `{}` command embed has too many fields",
                command.name
            );
            for field in command.fields {
                check_length("field name", field.name, MAX_EMBED_FIELD_NAME_CHARS)?;
                check_length("field value", field.value, MAX_EMBED_FIELD_VALUE_CHARS)?;
            }
        }
        // Help lists every command as an embed field and as a select option.
        ensure!(
            self.commands.responses.len() <= MAX_EMBED_FIELDS.min(MAX_SELECT_OPTIONS),
            "too many response commands for the help embed and menu"
        );
        Ok(())
    }

    fn validate_copy(&self) -> Result<()> {
        check_length(
            "help description",
            self.commands.help.embed_description,
            MAX_EMBED_DESCRIPTION_CHARS,
        )?;
        check_length(
            "support redirect description",
            self.commands.support_context.description,
            MAX_EMBED_DESCRIPTION_CHARS,
        )?;
        if let Some(starter) = self.thread_starter {
            check_length(
                "support title",
                starter.support_title,
                MAX_EMBED_TITLE_CHARS,
            )?;
            check_length(
                "support description",
                starter.support_description,
                MAX_EMBED_DESCRIPTION_CHARS,
            )?;
            check_length(
                "priority title",
                starter.priority_title,
                MAX_EMBED_TITLE_CHARS,
            )?;
            check_length(
                "priority description",
                starter.priority_description,
                MAX_EMBED_DESCRIPTION_CHARS,
            )?;
        }
        for reply in self.message_replies {
            check_length("reply title", reply.title, MAX_EMBED_TITLE_CHARS)?;
            check_length(
                "reply description",
                reply.description,
                MAX_EMBED_DESCRIPTION_CHARS,
            )?;
        }
        Ok(())
    }

    fn validate_docs_page_prefix(&self) -> Result<()> {
        let Some(prefix) = self.chatbot.ai.docs_page_prefix else {
            return Ok(());
        };
        let url = reqwest::Url::parse(prefix)
            .with_context(|| format!("invalid documentation page prefix `{prefix}`"))?;
        ensure!(
            url.scheme() == "https" && url.host_str().is_some() && url.path().len() > 1,
            "documentation page prefix `{prefix}` must be an https URL with a path"
        );
        Ok(())
    }

    fn validate_paste_checks(&self) -> Result<()> {
        for check in self.checks.paste_checks {
            let pattern = Regex::new(check.pattern)
                .with_context(|| format!("invalid paste pattern `{}`", check.pattern))?;
            // captures_len counts the whole match as group 0.
            ensure!(
                pattern.captures_len() == 2,
                "paste pattern `{}` must have exactly one capture group",
                check.pattern
            );
            ensure!(
                check.raw_url.contains("{code}"),
                "paste URL `{}` must contain {{code}}",
                check.raw_url
            );
        }
        Ok(())
    }
}

fn check_length(what: &str, text: &str, max_chars: usize) -> Result<()> {
    let length = text.chars().count();
    ensure!(
        length <= max_chars,
        "{what} has {length} characters and exceeds Discord's limit of {max_chars}: {text:.40}..."
    );
    Ok(())
}

pub const PASTE_CHECKS: &[PasteCheck] = &[
    PasteCheck {
        pattern: r"https?://hastebin\.com/(?:raw/)?(\w+)(?:\.\w+)?",
        raw_url: "https://hastebin.com/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://hasteb\.in/(?:raw/)?(\w+)(?:\.\w+)?",
        raw_url: "https://hasteb.in/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://paste\.helpch\.at/(?:raw/)?(\w+)(?:\.\w+)?",
        raw_url: "https://paste.helpch.at/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://bytebin\.lucko\.me/(\w+)",
        raw_url: "https://bytebin.lucko.me/{code}",
    },
    PasteCheck {
        pattern: r"https?://pastes\.dev/(\w+)",
        raw_url: "https://bytebin.lucko.me/{code}",
    },
    PasteCheck {
        pattern: r"https?://paste\.lucko\.me/(?:raw/)?(\w+)(?:\.\w+)?",
        raw_url: "https://paste.lucko.me/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://pastebin\.com/(?:raw/)?(\w{2,})(?:\.\w+)?",
        raw_url: "https://pastebin.com/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://gist\.github\.com/([\w-]+/\w+|\w+)",
        raw_url: "https://gist.github.com/{code}/raw/",
    },
    PasteCheck {
        pattern: r"https?://mclo\.gs/(\w+)",
        raw_url: "https://api.mclo.gs/1/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://gitlab\.com/snippets/(\w+)(?:\.\w+)?",
        raw_url: "https://gitlab.com/snippets/{code}/raw",
    },
];
