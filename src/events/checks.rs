use std::sync::LazyLock;

use anyhow::{Context as _, Result, anyhow};
use poise::serenity_prelude as serenity;
use regex::Regex;
use semver::Version;
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization as _;

use crate::{releases::LatestRelease, state::AppState};

const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

struct PasteMatch {
    raw_url: String,
    original_url: String,
}

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    let latest_release = if let Some(url) = data.bot.checks.release_url {
        data.services.releases.get(url).await
    } else {
        None
    };

    if let Some(found) = find_paste(message, data)? {
        handle_paste(ctx, data, message, found, latest_release.as_ref()).await?;
    }
    handle_images(ctx, data, message, latest_release.as_ref()).await
}

fn find_paste(message: &serenity::Message, data: &AppState) -> Result<Option<PasteMatch>> {
    let embed_fields = message
        .embeds
        .iter()
        .flat_map(|embed| embed.fields.iter().map(|field| field.value.as_str()));

    for check in data.bot.checks.paste_checks {
        let regex = Regex::new(check.pattern)
            .with_context(|| format!("invalid paste check regex {}", check.pattern))?;
        for content in std::iter::once(message.content.as_str()).chain(embed_fields.clone()) {
            let Some(captures) = regex.captures(content) else {
                continue;
            };
            let Some(code) = captures.get(1) else {
                continue;
            };
            let Some(original_url) = captures.get(0) else {
                continue;
            };
            return Ok(Some(PasteMatch {
                raw_url: check.raw_url.replace("{code}", code.as_str()),
                original_url: original_url.as_str().to_owned(),
            }));
        }
    }
    Ok(None)
}

async fn handle_paste(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    found: PasteMatch,
    latest_release: Option<&LatestRelease>,
) -> Result<()> {
    tracing::info!(url = %found.raw_url, "fetching paste");
    let response = data
        .services
        .http
        .get(&found.raw_url)
        .send()
        .await
        .context("failed to fetch paste")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        let embed = serenity::CreateEmbed::new()
            .title("Invalid Paste!")
            .colour(0xFF_00_00)
            .description(
                "The paste link you sent is invalid or expired. Check the link or create a new paste.",
            )
            .footer(serenity::CreateEmbedFooter::new(format!(
                "{} | Sent by {}",
                found.original_url, message.author.name
            )));
        reply(ctx, message, serenity::CreateMessage::new().embed(embed)).await?;
        return Ok(());
    }

    let text = response
        .error_for_status()
        .context("paste request failed")?
        .text()
        .await
        .context("failed to read paste")?;
    if text.is_empty() {
        return Ok(());
    }

    respond_to_text(
        ctx,
        data,
        message,
        &text,
        &format!("{} | Sent by {}", found.original_url, message.author.name),
        latest_release,
    )
    .await
}

async fn handle_images(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    latest_release: Option<&LatestRelease>,
) -> Result<()> {
    let attachments = message.attachments.iter().filter(|attachment| {
        attachment
            .content_type
            .as_deref()
            .is_some_and(|content_type| IMAGE_TYPES.contains(&content_type))
    });
    let mut recognized_any = false;

    for attachment in attachments {
        if u64::from(attachment.size) > MAX_IMAGE_BYTES {
            tracing::warn!(
                attachment = %attachment.filename,
                size = attachment.size,
                "skipping oversized OCR attachment"
            );
            continue;
        }

        let bytes = data
            .services
            .http
            .get(&attachment.proxy_url)
            .send()
            .await
            .context("failed to download OCR attachment")?
            .error_for_status()
            .context("OCR attachment download failed")?
            .bytes()
            .await
            .context("failed to read OCR attachment")?
            .to_vec();
        let text = data.services.ocr.recognize(bytes).await?;

        if contains_prohibited_ocr_content(&text) {
            delete_and_kick_ocr_spammer(ctx, message).await;
            return Ok(());
        }

        recognized_any = true;
        respond_to_text(
            ctx,
            data,
            message,
            &text,
            &format!("Sent by {}", message.author.name),
            latest_release,
        )
        .await?;
    }

    if recognized_any {
        message
            .react(ctx, serenity::ReactionType::Unicode("👀".to_owned()))
            .await?;
    }
    Ok(())
}

fn contains_prohibited_ocr_content(text: &str) -> bool {
    let normalized = normalize_ocr_moderation_text(text);
    normalized.contains("crypto") && normalized.contains("casino")
}

fn normalize_ocr_moderation_text(text: &str) -> String {
    text.nfkd()
        .flat_map(char::to_lowercase)
        .filter_map(|character| {
            let character = match character {
                '$' | '5' => 's',
                '!' | '1' | '|' | 'ι' | 'і' => 'i',
                '+' | '7' | 'τ' | 'т' => 't',
                '0' | 'ο' | 'о' => 'o',
                '3' | 'ε' | 'е' => 'e',
                '4' | '@' | 'α' | 'а' => 'a',
                'ρ' | 'р' => 'p',
                'с' => 'c',
                'υ' | 'у' => 'y',
                other => other,
            };
            character.is_ascii_lowercase().then_some(character)
        })
        .collect()
}

async fn delete_and_kick_ocr_spammer(ctx: &serenity::Context, message: &serenity::Message) {
    let Some(guild_id) = message.guild_id else {
        return;
    };
    let reason = format!(
        "OCR detected crypto/casino spam from {} ({})",
        message.author.tag(),
        message.author.id
    );

    if let Err(error) = message.delete(ctx).await {
        tracing::error!(%error, "failed to delete OCR spam message");
    }
    if let Err(error) = guild_id
        .kick_with_reason(&ctx.http, message.author.id, &reason)
        .await
    {
        tracing::error!(%error, "failed to kick OCR spam sender");
    }
}

async fn respond_to_text(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    text: &str,
    footer: &str,
    latest_release: Option<&LatestRelease>,
) -> Result<()> {
    for check in data.bot.checks.text_checks {
        if !text.contains(check.needle) {
            continue;
        }

        let mut embed = serenity::CreateEmbed::new()
            .title(check.title)
            .description(check.content)
            .colour(data.bot.accent_color)
            .footer(serenity::CreateEmbedFooter::new(footer))
            .field("Caused By", format!("```{}```", check.needle), false);
        for (index, tip) in check.tips.iter().enumerate() {
            embed = embed.field(format!("Tip #{}", index + 1), *tip, false);
        }
        if let Some(link) = check.link {
            embed = embed.field("Read More", link, false);
        }
        reply(ctx, message, serenity::CreateMessage::new().embed(embed)).await?;
    }

    if data.bot.checks.analyze_dump {
        analyze_dump(ctx, message, text, footer, latest_release).await?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct SkinsRestorerDump {
    #[serde(rename = "buildInfo")]
    build: BuildInfo,
    #[serde(rename = "environmentInfo")]
    environment: EnvironmentInfo,
    #[serde(rename = "javaInfo")]
    java: JavaInfo,
    #[serde(rename = "osInfo")]
    os: OsInfo,
    #[serde(rename = "platformInfo")]
    platform: PlatformInfo,
    #[serde(rename = "pluginInfo")]
    plugin: PluginInfo,
    #[serde(rename = "userInfo")]
    user: UserInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildInfo {
    build_time: String,
    version: String,
}

#[derive(Deserialize)]
struct EnvironmentInfo {
    hybrid: bool,
    platform: String,
    #[serde(rename = "platformType")]
    platform_type: String,
}

#[derive(Deserialize)]
struct JavaInfo {
    version: String,
}

#[derive(Deserialize)]
struct OsInfo {
    arch: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    platform_name: String,
    platform_version: String,
    plugins: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginInfo {
    config_data: ConfigData,
    proxy_mode: bool,
}

#[derive(Deserialize)]
struct ConfigData {
    dev: DevConfig,
}

#[derive(Deserialize)]
struct DevConfig {
    debug: bool,
}

#[derive(Deserialize)]
struct UserInfo {
    dir: String,
    home: String,
    name: String,
}

async fn analyze_dump(
    ctx: &serenity::Context,
    message: &serenity::Message,
    text: &str,
    footer: &str,
    latest_release: Option<&LatestRelease>,
) -> Result<()> {
    let Ok(dump) = serde_json::from_str::<SkinsRestorerDump>(text) else {
        return Ok(());
    };
    let raw_dump = serde_json::from_str::<serde_json::Value>(text)?;
    let embeds = build_dump_embeds(&dump, latest_release);
    let pretty_dump =
        serde_json::to_vec_pretty(&raw_dump).context("failed to format SkinsRestorer dump")?;
    reply(
        ctx,
        message,
        serenity::CreateMessage::new()
            .content(format!("Found the following for: `{footer}`"))
            .embeds(embeds)
            .add_file(serenity::CreateAttachment::bytes(pretty_dump, "dump.json")),
    )
    .await
}

fn build_dump_embeds(
    dump: &SkinsRestorerDump,
    latest_release: Option<&LatestRelease>,
) -> Vec<serenity::CreateEmbed> {
    let version = coerce_version(&dump.build.version);
    let latest_version = latest_release.and_then(|release| coerce_version(&release.tag_name));
    let version_display = version
        .as_ref()
        .map_or_else(|| dump.build.version.clone(), ToString::to_string);
    let mut embeds = Vec::new();

    if let (Some(version), Some(latest_version)) = (&version, &latest_version)
        && version < latest_version
    {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Important: Outdated SkinsRestorer Version!")
                .colour(0xED_42_45)
                .description(format!(
                    "The SkinsRestorer version you're using (`{version}`) is outdated! Please update to the latest version: `{latest_version}`"
                ))
                .field(
                    " ",
                    format!(
                        "[Download {latest_version}](https://modrinth.com/plugin/skinsrestorer/version/{latest_version})"
                    ),
                    false,
                ),
        );
    }

    embeds.push(
        serenity::CreateEmbed::new()
            .title("Info: Build")
            .colour(0x58_65_F2)
            .description(format!(
                "You are running version `{version_display}` of SkinsRestorer, built on `{}`.",
                dump.build.build_time
            )),
    );

    if dump.user.name == "?"
        || dump.user.dir == "/home/container"
        || dump.user.home == "/home/container"
    {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Info: Docker detected")
                .colour(0x58_65_F2)
                .description(
                    "We detected you are running SkinsRestorer in a Docker container and likely using a panel like Pterodactyl/Pelican. This is not an error, but we need to know this to better help you.",
                ),
        );
    }

    embeds.push(
        serenity::CreateEmbed::new()
            .title("Info: OS/Java")
            .colour(0x58_65_F2)
            .description(format!(
                "We detected you are running SkinsRestorer on `{}` with arch `{}` and Java `{}`",
                dump.os.name, dump.os.arch, dump.java.version
            )),
    );
    embeds.push(
        serenity::CreateEmbed::new()
            .title("Info: Platform/Environment")
            .colour(0x58_65_F2)
            .description(format!(
                "The dump is from the platform `{}` (`{}` & `{}`) with version `{}` and `{}` plugins.",
                dump.platform.platform_name,
                dump.environment.platform,
                dump.environment.platform_type,
                dump.platform.platform_version,
                dump.platform.plugins.len()
            )),
    );
    if dump.environment.hybrid {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Warning: Hybrid detected!")
                .colour(0xED_42_45)
                .description(
                    "The platform appears to be a hybrid platform, mixing mods with plugins. This is not supported and may cause issues.",
                ),
        );
    }
    embeds.push(
        serenity::CreateEmbed::new()
            .title("Info: Plugin")
            .colour(0x58_65_F2)
            .description(format!(
                "You are in proxy mode: `{}`, debug enabled: `{}`",
                dump.plugin.proxy_mode, dump.plugin.config_data.dev.debug
            )),
    );
    embeds
}

fn coerce_version(input: &str) -> Option<Version> {
    static VERSION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?P<major>\d+)\.(?P<minor>\d+)(?:\.(?P<patch>\d+))?")
            .expect("version regex is valid")
    });
    let captures = VERSION_PATTERN.captures(input)?;
    let major = captures.name("major")?.as_str();
    let minor = captures.name("minor")?.as_str();
    let patch = captures
        .name("patch")
        .map_or("0", |capture| capture.as_str());
    Version::parse(&format!("{major}.{minor}.{patch}")).ok()
}

async fn reply(
    ctx: &serenity::Context,
    message: &serenity::Message,
    builder: serenity::CreateMessage,
) -> Result<()> {
    message
        .channel_id
        .send_message(ctx, builder.reference_message(message))
        .await
        .map_err(|error| anyhow!(error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{coerce_version, contains_prohibited_ocr_content, normalize_ocr_moderation_text};

    #[test]
    fn normalizes_common_ocr_confusables() {
        assert_eq!(
            normalize_ocr_moderation_text("CRΥPT0 C@S1N0"),
            "cryptocasino"
        );
        assert!(contains_prohibited_ocr_content("CRYPTO\nCASINO"));
    }

    #[test]
    fn coerces_release_versions() {
        assert_eq!(
            coerce_version("v15.2.1-SNAPSHOT"),
            Some(semver::Version::new(15, 2, 1))
        );
    }
}
