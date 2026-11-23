#![warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use std::io;

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use unicode_normalization::{UnicodeNormalization as _, char::is_combining_mark};

use crate::{config::DumpAnalyzer, download, releases::LatestRelease, state::AppState};

use super::{
    honeypot,
    paste::{self, PasteLink},
};

const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_OCR_IMAGES_PER_MESSAGE: usize = 4;
const MAX_PASTE_BYTES: usize = 20 * 1024 * 1024;
// Discord's default upload limit is 10 MiB. Leave room for the rest of the message.
const MAX_PRETTY_DUMP_BYTES: usize = 8 * 1024 * 1024;

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

    if let Some(found) = paste::find_all(message, data).into_iter().next()
        && let Err(error) = handle_paste(ctx, data, message, found, latest_release.as_ref()).await
    {
        tracing::error!(
            error = %format_args!("{error:#}"),
            bot = data.bot.id,
            message_id = %message.id,
            "paste analysis failed"
        );
    }
    handle_images(ctx, data, message, latest_release.as_ref()).await
}

async fn handle_paste(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
    found: PasteLink,
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
    // Private, deleted, and expired pastes return 401, 403, 404, or 410.
    if response.status().is_client_error() {
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

    let response = response
        .error_for_status()
        .context("paste request failed")?;
    let text = download::read_limited_text(response, MAX_PASTE_BYTES, "paste response")
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
    let attachments = message
        .attachments
        .iter()
        .filter(|attachment| {
            attachment
                .content_type
                .as_deref()
                .is_some_and(|content_type| IMAGE_TYPES.contains(&content_type))
        })
        .take(MAX_OCR_IMAGES_PER_MESSAGE);
    let mut recognized_texts = Vec::new();

    for attachment in attachments {
        if u64::from(attachment.size) > MAX_IMAGE_BYTES {
            tracing::warn!(
                attachment = %attachment.filename,
                size = attachment.size,
                "skipping oversized OCR attachment"
            );
            continue;
        }

        // One unreadable image must not stop the spam check for the others.
        let text = match recognize_attachment(data, attachment).await {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(
                    error = format!("{error:#}"),
                    attachment = %attachment.filename,
                    message_id = %message.id,
                    "skipping unreadable OCR attachment"
                );
                continue;
            }
        };

        if contains_prohibited_ocr_content(&text) {
            delete_and_kick_ocr_spammer(ctx, data, message).await;
            return Ok(());
        }

        recognized_texts.push(text);
    }

    // Check all screenshots together so each match is answered once per message.
    if !recognized_texts.is_empty() {
        respond_to_text(
            ctx,
            data,
            message,
            &recognized_texts.join("\n"),
            &format!("Sent by {}", message.author.name),
            latest_release,
        )
        .await?;
    }

    if !recognized_texts.is_empty() {
        message
            .react(ctx, serenity::ReactionType::Unicode("👀".to_owned()))
            .await?;
    }
    Ok(())
}

async fn recognize_attachment(
    data: &AppState,
    attachment: &serenity::Attachment,
) -> Result<String> {
    let _slot = data.services.ocr.reserve().await?;
    let response = data
        .services
        .http
        .get(&attachment.proxy_url)
        .send()
        .await
        .context("failed to download OCR attachment")?
        .error_for_status()
        .context("OCR attachment download failed")?;
    let bytes = download::read_limited_bytes(
        response,
        usize::try_from(MAX_IMAGE_BYTES).unwrap_or(usize::MAX),
        "OCR attachment",
    )
    .await?;
    // Take the OCR slot only after the download so slow transfers do not block OCR.
    let permit = data.services.ocr.acquire().await?;
    data.services.ocr.recognize(bytes, permit).await
}

fn contains_prohibited_ocr_content(text: &str) -> bool {
    // Match within words so unrelated text cannot join across word breaks.
    let normalized = normalize_ocr_moderation_text(text);
    let has_word = |needle: &str| normalized.split(' ').any(|word| word.contains(needle));
    has_word("crypto") && has_word("casino")
}

/// Lowercases text, maps common confusables to ASCII letters, and separates
/// words with single spaces.
fn normalize_ocr_moderation_text(text: &str) -> String {
    let mapped = text
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .map(|character| match character {
            '$' | '5' => 's',
            '!' | '1' | '|' | 'ι' | 'і' => 'i',
            '+' | '7' | 'τ' | 'т' => 't',
            '0' | 'ο' | 'о' => 'o',
            '3' | 'ε' | 'е' => 'e',
            '4' | '@' | 'α' | 'а' => 'a',
            'ρ' | 'р' => 'p',
            'с' => 'c',
            'υ' | 'у' => 'y',
            other if other.is_ascii_lowercase() => other,
            _ => ' ',
        })
        .collect::<String>();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn delete_and_kick_ocr_spammer(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) {
    let Some(guild_id) = message.guild_id else {
        return;
    };
    match honeypot::is_exempt_member(
        ctx,
        guild_id,
        message.author.id,
        data.bot
            .honeypot
            .map_or(&[][..], |honeypot| honeypot.exempt_role_ids),
    )
    .await
    {
        Ok(false) => {}
        Ok(true) => {
            tracing::info!(
                message_id = %message.id,
                user_id = %message.author.id,
                "OCR spam match from exempt member ignored"
            );
            return;
        }
        Err(error) => {
            tracing::error!(
                error = format!("{error:#}"),
                message_id = %message.id,
                "skipping OCR spam action because the exemption check failed"
            );
            return;
        }
    }
    let reason = format!(
        "OCR detected crypto/casino spam from {} ({})",
        message.author.tag(),
        message.author.id
    );

    if let Err(error) = message.delete(ctx).await {
        tracing::error!(error = %format_args!("{error:#}"), "failed to delete OCR spam message");
    }
    if let Err(error) = guild_id
        .kick_with_reason(&ctx.http, message.author.id, &reason)
        .await
    {
        tracing::error!(error = %format_args!("{error:#}"), "failed to kick OCR spam sender");
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
            embed = embed.field(format!("Tip #{}", index.saturating_add(1)), *tip, false);
        }
        if let Some(link) = check.link {
            embed = embed.field("Read More", link, false);
        }
        // One failed reply must not hide the remaining matches.
        if let Err(error) = reply(ctx, message, serenity::CreateMessage::new().embed(embed)).await {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                message_id = %message.id,
                needle = check.needle,
                "failed to send a text check reply"
            );
        }
    }

    if let Some(analyzer) = data.bot.checks.dump_analyzer {
        analyze_dump(ctx, message, analyzer, text, footer, latest_release).await?;
    }
    Ok(())
}

async fn analyze_dump(
    ctx: &serenity::Context,
    message: &serenity::Message,
    analyzer: DumpAnalyzer,
    text: &str,
    footer: &str,
    latest_release: Option<&LatestRelease>,
) -> Result<()> {
    // Parsing and formatting a dump of up to 20 MB is CPU work, so keep it off
    // the async executor threads.
    let text = text.to_owned();
    let latest_release = latest_release.cloned();
    let analysis = tokio::task::spawn_blocking(move || {
        let embed = analyzer(&text, latest_release.as_ref())?;
        Some((embed, pretty_print_json(&text)))
    })
    .await
    .context("dump analysis task failed")?;
    let Some((embed, pretty_dump)) = analysis else {
        return Ok(());
    };

    let mut reply_message = serenity::CreateMessage::new().embed(embed);
    // Send the analysis even when the formatted dump is too large to attach.
    match pretty_dump {
        Ok(pretty_dump) => {
            reply_message = reply_message
                .content(format!("Analysis for `{footer}`:"))
                .add_file(serenity::CreateAttachment::bytes(pretty_dump, "dump.json"));
        }
        Err(error) => {
            tracing::info!(
                error = %format_args!("{error:#}"),
                message_id = %message.id,
                "sending dump analysis without the formatted dump"
            );
            reply_message = reply_message.content(format!(
                "Analysis for `{footer}` (the formatted dump is too large to attach):"
            ));
        }
    }
    reply(ctx, message, reply_message).await
}

fn pretty_print_json(text: &str) -> Result<Vec<u8>> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let mut output =
        LimitedWriter::new(text.len().min(MAX_PRETTY_DUMP_BYTES), MAX_PRETTY_DUMP_BYTES);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    serde_transcode::transcode(&mut deserializer, &mut serializer)
        .context("failed to format the dump")?;
    deserializer
        .end()
        .context("the dump contains trailing data")?;
    Ok(output.into_inner())
}

struct LimitedWriter {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl LimitedWriter {
    fn new(capacity: usize, max_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
            max_bytes,
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl io::Write for LimitedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(buffer.len())
            .is_none_or(|length| length > self.max_bytes)
        {
            return Err(io::Error::other("formatted dump exceeds attachment limit"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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
        .context("failed to reply to the message")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::{
        contains_prohibited_ocr_content, normalize_ocr_moderation_text, pretty_print_json,
    };

    #[test]
    fn normalizes_common_ocr_confusables() -> Result<()> {
        ensure!(normalize_ocr_moderation_text("CRΥPT0 C@S1N0") == "crypto casino");
        ensure!(normalize_ocr_moderation_text("Café-Bar") == "cafe bar");
        ensure!(contains_prohibited_ocr_content("CRYPTO\nCASINO"));
        ensure!(contains_prohibited_ocr_content("Best CryptoCasino bonus"));
        Ok(())
    }

    #[test]
    fn ignores_crypto_and_casino_split_across_words() -> Result<()> {
        ensure!(!contains_prohibited_ocr_content(
            "javax.crypto.BadPaddingException\nSync basic as in old versions"
        ));
        // Both letter runs exist here, but only across word breaks.
        ensure!(!contains_prohibited_ocr_content("crypto ca sino"));
        ensure!(!contains_prohibited_ocr_content("cryp to casino"));
        ensure!(!contains_prohibited_ocr_content("crypto"));
        ensure!(!contains_prohibited_ocr_content("casino"));
        Ok(())
    }

    #[test]
    fn formats_json_dumps_without_rejecting_escaped_paths() -> Result<()> {
        let input = r#"{
            "buildInfo":{"buildTime":"today","version":"15.2.1"},
            "environmentInfo":{"hybrid":false,"platform":"Paper","platformType":"SERVER"},
            "javaInfo":{"version":"21"},
            "osInfo":{"arch":"x86_64","name":"Windows"},
            "platformInfo":{"platformName":"Paper","platformVersion":"1.21","plugins":[{"name":"A"}]},
            "pluginInfo":{"configData":{"dev":{"debug":false}},"proxyMode":false},
            "userInfo":{"dir":"C:\\server","home":"C:\\Users\\Alex","name":"Alex"}
        }"#;

        let formatted = pretty_print_json(input)?;
        let original_value = serde_json::from_str::<serde_json::Value>(input)?;
        let formatted_value = serde_json::from_slice::<serde_json::Value>(&formatted)?;
        ensure!(formatted_value == original_value);
        Ok(())
    }
}
