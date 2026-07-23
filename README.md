# Steward

Steward runs the SkinsRestorer and SoulFire support bots in one Rust process.
It provides static support commands, message logging, paste uploads, support
thread replies, moderation checks, AI-assisted answers, and local image OCR.

Bot definitions live in [`src/bots`](src/bots). Each bot has a typed Rust
configuration for its identity, commands, responses, prompts, checks, and
support links.

## Requirements

- Rust 1.95 or newer
- Discord applications with the Message Content and Server Members privileged
  intents enabled
- A token for each configured bot
- DeepSeek and Brave Search API keys for AI support replies

## Configuration

Set these environment variables in the shell, a local `.env` file, or the
deployment environment:

| Variable | Purpose |
| --- | --- |
| `DISCORD_TOKEN_STEWARD` | Discord token for Steward |
| `DISCORD_TOKEN_JARVIS` | Discord token for Jarvis |
| `DEEPSEEK_API_KEY` | DeepSeek access for support answers |
| `BRAVE_SEARCH_API_KEY` | Current web context for support answers |
| `RUST_LOG` | Optional log filter, such as `steward=debug,info` |

Both Discord tokens are required because the process starts both bots
together. The AI keys are read when an AI support reply is requested.

## Run locally

```bash
cargo run --release
```

Release mode matters for OCR performance. The detection and recognition models
are embedded into the executable at build time, so no model download or runtime
model directory is needed.

## Validate changes

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

## Deploy with Railpack

Railpack detects the root `Cargo.toml` as a Rust application, builds the
`steward` release binary, and starts it as `./bin/steward`. The repository does
not need a Railpack configuration file, Dockerfile, or Compose file.

Configure the environment variables above in the deployment service and deploy
the repository root.

## Add or change a bot

Edit the matching file in [`src/bots`](src/bots). Add a new definition to
[`src/bots/mod.rs`](src/bots/mod.rs) when introducing another bot. Commands are
registered from these static definitions at startup.

The bundled OCR model provenance and license are documented in
[`models/README.md`](models/README.md).
