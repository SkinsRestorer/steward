use poise::serenity_prelude as serenity;

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

#[derive(Clone, Copy, Debug)]
pub struct ChecksConfig {
    pub paste_checks: &'static [PasteCheck],
    pub release_url: Option<&'static str>,
    pub text_checks: &'static [TextCheck],
    pub analyze_dump: bool,
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
pub struct ThreadStarterConfig {
    pub support_title: &'static str,
    pub support_description: &'static str,
    pub support_banner_url: &'static str,
    pub support_gpt_url: &'static str,
    pub docs_url: &'static str,
    pub priority_title: &'static str,
    pub priority_description: &'static str,
    pub priority_banner_url: &'static str,
    pub pricing_url: &'static str,
    pub priority_support_url: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct BotDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub token_env: &'static str,
    pub application_id: u64,
    pub accent_color: u32,
    pub logs_dir: &'static str,
    pub presence: &'static str,
    pub autoupload: AutouploadConfig,
    pub chatbot: ChatbotConfig,
    pub checks: ChecksConfig,
    pub commands: CommandsConfig,
    pub no_ping: NoPingConfig,
    pub thread_starter: ThreadStarterConfig,
    pub message_replies: bool,
}

pub const PASTE_CHECKS: &[PasteCheck] = &[
    PasteCheck {
        pattern: r"https?://hastebin\.com/(\w+)(?:\.\w+)?",
        raw_url: "https://hastebin.com/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://hasteb\.in/(\w+)(?:\.\w+)?",
        raw_url: "https://hasteb.in/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://paste\.helpch\.at/(\w+)(?:\.\w+)?",
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
        pattern: r"https?://paste\.lucko\.me/(\w+)(?:\.\w+)?",
        raw_url: "https://paste.lucko.me/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://pastebin\.com/(\w+)(?:\.\w+)?",
        raw_url: "https://pastebin.com/raw/{code}",
    },
    PasteCheck {
        pattern: r"https?://gist\.github\.com/(\w+/\w+)(?:\.\w+/\w+)?",
        raw_url: "https://gist.github.com/{code}/raw/",
    },
    PasteCheck {
        pattern: r"https?://gitlab\.com/snippets/(\w+)(?:\.\w+)?",
        raw_url: "https://gitlab.com/snippets/{code}/raw",
    },
];
