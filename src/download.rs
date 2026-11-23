#![warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use anyhow::{Context as _, Result, bail};
use futures::StreamExt as _;

pub async fn read_limited_bytes(
    response: reqwest::Response,
    max_bytes: usize,
    resource: &str,
) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        bail!("{resource} exceeds the {max_bytes} byte limit");
    }

    let initial_capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .map_or(0, |length| length.min(max_bytes));
    let mut bytes = Vec::with_capacity(initial_capacity);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("failed to read {resource}"))?;
        let next_length = bytes
            .len()
            .checked_add(chunk.len())
            .context("response length overflowed usize")?;
        if next_length > max_bytes {
            bail!("{resource} exceeds the {max_bytes} byte limit");
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

pub async fn read_limited_text(
    response: reqwest::Response,
    max_bytes: usize,
    resource: &str,
) -> Result<String> {
    let bytes = read_limited_bytes(response, max_bytes, resource).await?;
    Ok(decode_text(&bytes))
}

// Windows tools such as PowerShell 5 write UTF-16 files with a byte order
// mark. Decoding those bytes as UTF-8 puts a NUL between every character.
fn decode_text(bytes: &[u8]) -> String {
    if let Some(text) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        return String::from_utf8_lossy(text).into_owned();
    }
    if let Some(text) = bytes.strip_prefix(b"\xFF\xFE") {
        return decode_utf16(text, u16::from_le_bytes);
    }
    if let Some(text) = bytes.strip_prefix(b"\xFE\xFF") {
        return decode_utf16(text, u16::from_be_bytes);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_utf16(bytes: &[u8], to_unit: fn([u8; 2]) -> u16) -> String {
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| to_unit(*pair))
        .collect::<Vec<_>>();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::decode_text;

    #[test]
    fn decodes_text_by_byte_order_mark() -> Result<()> {
        let little_endian = [0xFF, 0xFE, b'O', 0, b'K', 0, 0x3D, 0xD8, 0x00, 0xDE];
        let big_endian = [0xFE, 0xFF, 0, b'O', 0, b'K'];
        ensure!(decode_text(&little_endian) == "OK\u{1F600}");
        ensure!(decode_text(&big_endian) == "OK");
        ensure!(decode_text(b"\xEF\xBB\xBF{\"ok\":true}") == r#"{"ok":true}"#);
        ensure!(decode_text("plain \u{e9}".as_bytes()) == "plain \u{e9}");
        Ok(())
    }
}
