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
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
