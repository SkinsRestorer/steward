#![warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

//! Ordered chat history and the limits that bound it.
//!
//! Each user turn takes an even order and its reply takes the next odd order,
//! so a reply sorts right after the message it answers even when newer
//! messages arrived during generation.

use std::collections::VecDeque;

use crate::ai::ChatMessage;

use super::{MAX_HISTORY_BYTES, MAX_HISTORY_IMAGES, MAX_HISTORY_MESSAGES};

/// History sent to the model before older turns are compacted.
pub(super) const ACTIVE_HISTORY: HistoryLimits = HistoryLimits {
    bytes: MAX_HISTORY_BYTES,
    messages: MAX_HISTORY_MESSAGES,
    images: MAX_HISTORY_IMAGES,
};
/// History kept verbatim after a compaction.
pub(super) const RECENT_HISTORY: HistoryLimits = HistoryLimits {
    bytes: MAX_HISTORY_BYTES / 2,
    messages: MAX_HISTORY_MESSAGES / 2,
    images: MAX_HISTORY_IMAGES / 2,
};
/// Hard bound for queued turns and retries, even when the model is unavailable.
pub(super) const BUFFERED_HISTORY: HistoryLimits = HistoryLimits {
    bytes: MAX_HISTORY_BYTES * 4,
    messages: MAX_HISTORY_MESSAGES * 4,
    images: MAX_HISTORY_IMAGES * 4,
};

#[derive(Clone, Copy)]
pub(super) struct HistoryLimits {
    pub(super) bytes: usize,
    pub(super) messages: usize,
    pub(super) images: usize,
}

pub(super) struct HistoryEntry {
    pub(super) message: ChatMessage,
    pub(super) order: u64,
}

/// Inserts a reply right after the user turn it answers. Does nothing when
/// that turn was already trimmed or compacted away.
pub(super) fn insert_reply(
    messages: &mut VecDeque<HistoryEntry>,
    request_order: u64,
    message: ChatMessage,
) {
    if !messages.iter().any(|entry| entry.order == request_order) {
        return;
    }
    let entry = HistoryEntry {
        message,
        order: request_order.saturating_add(1),
    };
    let index = messages
        .iter()
        .position(|candidate| candidate.order > entry.order)
        .unwrap_or(messages.len());
    messages.insert(index, entry);
}

pub(super) fn trim_history(messages: &mut VecDeque<HistoryEntry>) {
    let dropped = excess_prefix_len(messages, BUFFERED_HISTORY);
    if dropped > 0 {
        messages.drain(..dropped);
        tracing::warn!(
            dropped,
            "support history reached the hard buffer limit before compaction"
        );
    }
}

/// Returns how many leading entries to drop so the rest fits `limits`. Drops
/// whole turns and always keeps the latest user turn, even when it alone
/// exceeds the limits.
pub(super) fn excess_prefix_len(messages: &VecDeque<HistoryEntry>, limits: HistoryLimits) -> usize {
    let Some(last_user) = messages
        .iter()
        .rposition(|entry| matches!(entry.message, ChatMessage::User(_, _)))
    else {
        return 0;
    };
    let mut bytes = messages
        .iter()
        .map(|entry| entry.message.len())
        .fold(0_usize, usize::saturating_add);
    let mut images = messages
        .iter()
        .map(|entry| entry.message.image_count())
        .fold(0_usize, usize::saturating_add);
    let mut remaining = messages.len();
    let mut entries = messages.iter().take(last_user).peekable();
    let mut start = 0_usize;

    while remaining > limits.messages || bytes > limits.bytes || images > limits.images {
        let Some(entry) = entries.next() else {
            break;
        };
        bytes = bytes.saturating_sub(entry.message.len());
        images = images.saturating_sub(entry.message.image_count());
        remaining = remaining.saturating_sub(1);
        start = start.saturating_add(1);
        // Drop the replies that belong to the dropped user turn as well.
        while let Some(reply) =
            entries.next_if(|entry| matches!(entry.message, ChatMessage::Assistant(_)))
        {
            bytes = bytes.saturating_sub(reply.message.len());
            remaining = remaining.saturating_sub(1);
            start = start.saturating_add(1);
        }
    }
    start
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Arc};

    use anyhow::{Result, ensure};

    use super::{ACTIVE_HISTORY, HistoryEntry, MAX_HISTORY_BYTES, excess_prefix_len, insert_reply};
    use crate::ai::ChatMessage;

    fn user(order: u64, bytes: usize) -> HistoryEntry {
        HistoryEntry {
            message: ChatMessage::User(Arc::from("u".repeat(bytes)), vec![]),
            order,
        }
    }

    fn reply(order: u64, bytes: usize) -> HistoryEntry {
        HistoryEntry {
            message: ChatMessage::Assistant(Arc::from("a".repeat(bytes))),
            order,
        }
    }

    #[test]
    fn keeps_history_within_the_limits() -> Result<()> {
        let messages = VecDeque::from([user(2, 10), reply(3, 10), user(4, 10)]);
        ensure!(excess_prefix_len(&messages, ACTIVE_HISTORY) == 0);
        Ok(())
    }

    #[test]
    fn keeps_a_latest_turn_that_alone_exceeds_the_limits() -> Result<()> {
        let messages = VecDeque::from([user(2, MAX_HISTORY_BYTES * 2)]);
        ensure!(excess_prefix_len(&messages, ACTIVE_HISTORY) == 0);
        Ok(())
    }

    #[test]
    fn drops_the_oldest_turn_together_with_its_reply() -> Result<()> {
        let third = MAX_HISTORY_BYTES / 3;
        let messages = VecDeque::from([
            user(2, third),
            reply(3, 10),
            user(4, third),
            reply(5, 10),
            user(6, third),
        ]);
        ensure!(excess_prefix_len(&messages, ACTIVE_HISTORY) == 2);
        Ok(())
    }

    #[test]
    fn compacts_only_when_the_byte_limit_is_exceeded() -> Result<()> {
        let half = MAX_HISTORY_BYTES / 2;
        let at_limit = VecDeque::from([user(2, half), user(4, MAX_HISTORY_BYTES - half)]);
        ensure!(excess_prefix_len(&at_limit, ACTIVE_HISTORY) == 0);
        let over_limit = VecDeque::from([user(2, half), user(4, MAX_HISTORY_BYTES - half + 1)]);
        ensure!(excess_prefix_len(&over_limit, ACTIVE_HISTORY) == 1);
        Ok(())
    }

    #[test]
    fn inserts_a_reply_only_while_its_turn_is_present() -> Result<()> {
        let mut messages = VecDeque::from([user(2, 1), user(4, 1)]);
        insert_reply(&mut messages, 2, ChatMessage::Assistant(Arc::from("reply")));
        let orders = messages.iter().map(|entry| entry.order).collect::<Vec<_>>();
        ensure!(orders == [2, 3, 4], "orders {orders:?}");

        insert_reply(
            &mut messages,
            8,
            ChatMessage::Assistant(Arc::from("orphan")),
        );
        ensure!(messages.len() == 3);
        Ok(())
    }
}
