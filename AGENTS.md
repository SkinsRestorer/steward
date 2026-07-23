# Agent Guidelines for Steward Discord Bot

## Build, lint, and test commands

- Start: `cargo run --release`
- Format: `cargo fmt --all`
- Check formatting: `cargo fmt --all --check`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-targets`
- Release build: `cargo build --release --locked`

Run the bot in release mode when evaluating OCR performance.

## Rust style

- Keep code compatible with the Rust version declared in `Cargo.toml`.
- Format all Rust code with `rustfmt`.
- Resolve every Clippy warning.
- Prefer typed configuration and enums over loosely structured values.
- Use `Result` and the `?` operator for recoverable failures.
- Add context when propagating errors across service boundaries.
- Use early returns for guard clauses.
- Keep blocking CPU work outside Tokio executor threads with
  `tokio::task::spawn_blocking`.
- Do not use `unsafe` code.

## Project structure

- `src/bots/` contains the static definition for each Discord bot.
- `src/events/` contains message and gateway event behavior.
- `src/commands.rs` builds Poise application commands.
- `src/ai.rs` owns DeepSeek generation and Brave Search integration.
- `src/ocr.rs` owns the embedded Ocrs model runtime.
- `src/releases.rs` owns release metadata caching.

Keep bot-specific copy, identifiers, prompts, and commands in the matching bot
definition. Put reusable runtime behavior in the shared service or event
modules.

## Tests

Add focused unit or integration tests for non-trivial parsing, normalization,
state transitions, and configuration limits. Avoid tests that only assert that
source or output contains a string.
