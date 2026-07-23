use anyhow::{Context as _, Result, bail};
use poise::serenity_prelude as serenity;
use serde::Deserialize;

use crate::state::AppState;

const SUPPORTED_CONTENT_TYPES: &[&str] = &[
    "application/json",
    "application/yaml",
    "text/xml",
    "text/plain",
];
const PASTE_API: &str = "https://api.pastes.dev/post";
const PASTE_WEBSITE: &str = "https://pastes.dev";
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Deserialize)]
struct PasteResponse {
    key: String,
}

pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<()> {
    for attachment in &message.attachments {
        let Some(content_type) = attachment.content_type.as_deref() else {
            continue;
        };
        if !SUPPORTED_CONTENT_TYPES
            .iter()
            .any(|supported| content_type.contains(supported))
        {
            continue;
        }

        match upload_attachment(data, attachment, content_type).await {
            Ok(uploaded_url) => {
                message
                    .reply(
                        ctx,
                        (data.bot.autoupload.future_uploads_message)(
                            &attachment.filename,
                            &uploaded_url,
                        ),
                    )
                    .await?;
            }
            Err(error) => {
                tracing::error!(%error, attachment = %attachment.filename, "automatic paste upload failed");
                message
                    .reply(ctx, data.bot.autoupload.failed_upload_message)
                    .await?;
            }
        }
    }

    Ok(())
}

async fn upload_attachment(
    data: &AppState,
    attachment: &serenity::Attachment,
    fallback_content_type: &str,
) -> Result<String> {
    if u64::from(attachment.size) > MAX_ATTACHMENT_BYTES {
        bail!("attachment exceeds the {MAX_ATTACHMENT_BYTES} byte upload limit");
    }

    let content = data
        .services
        .http
        .get(&attachment.url)
        .send()
        .await
        .context("failed to download text attachment")?
        .error_for_status()
        .context("text attachment download failed")?
        .text()
        .await
        .context("failed to read text attachment")?;
    let content_type = detect_text_format(&content).unwrap_or(fallback_content_type);
    let response = data
        .services
        .http
        .post(PASTE_API)
        .header("Content-Type", content_type)
        .header("User-Agent", data.bot.autoupload.user_agent)
        .body(content)
        .send()
        .await
        .context("failed to upload attachment to pastes.dev")?
        .error_for_status()
        .context("pastes.dev rejected attachment")?
        .json::<PasteResponse>()
        .await
        .context("failed to decode pastes.dev response")?;

    Ok(format!("{PASTE_WEBSITE}/{}", response.key))
}

fn detect_text_format(text: &str) -> Option<&'static str> {
    let text = text.trim();
    if text.starts_with('{')
        && text.ends_with('}')
        && serde_json::from_str::<serde_json::Value>(text).is_ok()
    {
        return Some("text/json");
    }
    if yaml_serde::from_str::<yaml_serde::Value>(text).is_ok() {
        return Some("text/yaml");
    }
    if text.starts_with('<') && text.ends_with('>') && roxmltree::Document::parse(text).is_ok() {
        return Some("text/xml");
    }
    None
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::detect_text_format;

    #[test]
    fn detects_structured_attachment_formats() -> Result<()> {
        ensure!(detect_text_format(r#"{"ok":true}"#) == Some("text/json"));
        ensure!(detect_text_format("key: value") == Some("text/yaml"));
        Ok(())
    }
}
