#![warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::ai::neutralize_prompt_tags;

use super::{
    APPLICATION_TEXT_TYPES, MAX_IMAGE_BYTES, MAX_IMAGE_URL_BYTES, MAX_MESSAGE_IMAGES,
    MAX_SOURCE_LABEL_BYTES, MAX_USER_MESSAGE_BYTES, SUPPORT_DATA_INTRO, SUPPORT_DATA_OUTRO,
    SUPPORT_DATA_SOURCE_END, SourceContent, TEXT_FILE_EXTENSIONS,
};

pub(super) fn is_text_attachment(attachment: &serenity::Attachment) -> bool {
    attachment
        .content_type
        .as_deref()
        .is_some_and(is_text_content_type)
        || is_text_filename(&attachment.filename)
}

pub(super) fn is_text_content_type(content_type: &str) -> bool {
    let content_type = content_type.split(';').next().map_or("", str::trim);
    content_type.starts_with("text/") || APPLICATION_TEXT_TYPES.contains(&content_type)
}

pub(super) fn is_text_filename(filename: &str) -> bool {
    filename.rsplit_once('.').is_some_and(|(_, extension)| {
        TEXT_FILE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
    })
}

pub(crate) fn collect_images(attachments: &[serenity::Attachment]) -> Vec<Arc<str>> {
    attachments
        .iter()
        .filter(|attachment| {
            let supported = attachment.content_type.as_deref().map_or_else(
                || {
                    attachment
                        .filename
                        .rsplit_once('.')
                        .is_some_and(|(_, extension)| {
                            matches!(
                                extension.to_ascii_lowercase().as_str(),
                                "png" | "jpg" | "jpeg" | "webp" | "gif"
                            )
                        })
                },
                |content_type| {
                    matches!(
                        content_type.split(';').next().unwrap_or_default().trim(),
                        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
                    )
                },
            );
            supported
                && attachment.size <= MAX_IMAGE_BYTES
                && !attachment.url.is_empty()
                && attachment.url.len() <= MAX_IMAGE_URL_BYTES
        })
        .take(MAX_MESSAGE_IMAGES)
        .map(|attachment| Arc::from(attachment.url.as_str()))
        .collect()
}

pub(super) fn build_content(
    user_message: &str,
    sources: &[SourceContent],
    max_bytes: usize,
) -> String {
    // Neutralize before excerpting so the byte budget still holds.
    let user_message = excerpt_text(
        &neutralize_prompt_tags(user_message),
        MAX_USER_MESSAGE_BYTES.min(max_bytes),
    );
    if sources.is_empty() {
        return user_message;
    }

    let labels = sources
        .iter()
        .map(|source| sanitize_source_label(&source.label))
        .collect::<Vec<_>>();
    let headers = labels
        .iter()
        .map(|label| format!("\n<source>\nSource: {label}\nContent:\n"))
        .collect::<Vec<_>>();
    let fixed_bytes = user_message
        .len()
        .saturating_add(SUPPORT_DATA_INTRO.len())
        .saturating_add(SUPPORT_DATA_OUTRO.len())
        .saturating_add(headers.iter().map(String::len).sum::<usize>())
        .saturating_add(SUPPORT_DATA_SOURCE_END.len().saturating_mul(sources.len()));
    let Some(mut remaining_content_bytes) = max_bytes
        .checked_sub(fixed_bytes)
        .filter(|&bytes| bytes > 0)
    else {
        return excerpt_text(&user_message, max_bytes);
    };
    let mut content = String::with_capacity(max_bytes);
    content.push_str(&user_message);
    content.push_str(SUPPORT_DATA_INTRO);
    for ((source, header), remaining_sources) in
        sources.iter().zip(headers).zip((1..=sources.len()).rev())
    {
        content.push_str(&header);
        let Some(source_budget) = remaining_content_bytes.checked_div(remaining_sources) else {
            return excerpt_text(&user_message, max_bytes);
        };
        let excerpt = excerpt_text(&neutralize_prompt_tags(source.text.trim()), source_budget);
        // Exhaust the budget when an excerpt uses the remaining bytes.
        remaining_content_bytes = remaining_content_bytes.saturating_sub(excerpt.len());
        content.push_str(&excerpt);
        content.push_str(SUPPORT_DATA_SOURCE_END);
    }
    content.push_str(SUPPORT_DATA_OUTRO);
    debug_assert!(
        content.len() <= max_bytes,
        "support content has {} bytes and exceeds the {max_bytes}-byte budget",
        content.len()
    );
    content
}

fn sanitize_source_label(label: &str) -> String {
    let sanitized = neutralize_prompt_tags(label)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    prefix_bytes(sanitized.trim(), MAX_SOURCE_LABEL_BYTES).to_owned()
}

fn excerpt_text(text: &str, max_bytes: usize) -> String {
    const OMITTED: &str = "\n...[content omitted]...\n";

    let text = text.trim();
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let Some(excerpt_bytes) = max_bytes
        .checked_sub(OMITTED.len())
        .filter(|&bytes| bytes > 0)
    else {
        return prefix_bytes(text, max_bytes).to_owned();
    };
    let head = prefix_bytes(text, excerpt_bytes / 3);
    let Some(tail_bytes) = excerpt_bytes.checked_sub(head.len()) else {
        return prefix_bytes(text, max_bytes).to_owned();
    };
    let tail = suffix_bytes(text, tail_bytes);
    format!("{head}{OMITTED}{tail}")
}

pub(super) fn prefix_bytes(text: &str, max_bytes: usize) -> &str {
    let end = text.floor_char_boundary(max_bytes);
    text.get(..end).unwrap_or_default()
}

pub(super) fn suffix_bytes(text: &str, max_bytes: usize) -> &str {
    // A budget larger than the string starts the suffix at the beginning.
    let start = text.ceil_char_boundary(text.len().saturating_sub(max_bytes));
    text.get(start..).unwrap_or_default()
}
