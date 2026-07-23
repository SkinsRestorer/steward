use anyhow::{Context as _, Result, bail};
use poise::serenity_prelude as serenity;
use serde::{
    Deserialize,
    de::{IgnoredAny, MapAccess, SeqAccess, Visitor},
};

use crate::{download, state::AppState};

const SUPPORTED_CONTENT_TYPES: &[&str] = &[
    "application/json",
    "application/yaml",
    "text/xml",
    "text/plain",
];
const PASTE_API: &str = "https://api.pastes.dev/post";
const PASTE_WEBSITE: &str = "https://pastes.dev";
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PASTE_RESPONSE_BYTES: usize = 64 * 1024;

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
        let Some(content_type) = attachment
            .content_type
            .as_deref()
            .and_then(|content_type| content_type.split(';').next())
            .map(str::trim)
        else {
            continue;
        };
        if !SUPPORTED_CONTENT_TYPES.contains(&content_type) {
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

    let response = data
        .services
        .http
        .get(&attachment.url)
        .send()
        .await
        .context("failed to download text attachment")?
        .error_for_status()
        .context("text attachment download failed")?;
    let content = download::read_limited_text(
        response,
        usize::try_from(MAX_ATTACHMENT_BYTES).unwrap_or(usize::MAX),
        "text attachment",
    )
    .await?;
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
        .context("pastes.dev rejected attachment")?;
    let response =
        download::read_limited_bytes(response, MAX_PASTE_RESPONSE_BYTES, "pastes.dev response")
            .await
            .context("failed to read pastes.dev response")?;
    let response = serde_json::from_slice::<PasteResponse>(&response)
        .context("failed to decode pastes.dev response")?;

    Ok(format!("{PASTE_WEBSITE}/{}", response.key))
}

fn detect_text_format(text: &str) -> Option<&'static str> {
    let text = text.trim();
    if ((text.starts_with('{') && text.ends_with('}'))
        || (text.starts_with('[') && text.ends_with(']')))
        && serde_json::from_str::<StructuredData>(text).is_ok()
    {
        return Some("application/json");
    }
    if text.starts_with('<') && text.ends_with('>') && roxmltree::Document::parse(text).is_ok() {
        return Some("text/xml");
    }
    if yaml_serde::from_str::<StructuredData>(text).is_ok() {
        return Some("application/yaml");
    }
    None
}

struct StructuredData;

impl<'de> Deserialize<'de> for StructuredData {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StructuredDataVisitor)
    }
}

struct StructuredDataVisitor;

impl<'de> Visitor<'de> for StructuredDataVisitor {
    type Value = StructuredData;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a mapping or sequence")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(StructuredData)
    }

    fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(StructuredData)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::detect_text_format;

    #[test]
    fn detects_structured_attachment_formats() -> Result<()> {
        ensure!(detect_text_format(r#"{"ok":true}"#) == Some("application/json"));
        ensure!(detect_text_format(r#"[{"ok":true}]"#) == Some("application/json"));
        ensure!(detect_text_format("key: value") == Some("application/yaml"));
        ensure!(detect_text_format("<root>value</root>") == Some("text/xml"));
        ensure!(detect_text_format("ordinary plain text").is_none());
        Ok(())
    }
}
