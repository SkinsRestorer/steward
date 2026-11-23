# Steward

Steward runs the SkinsRestorer and SoulFire support bots in one Rust process.
It provides static support commands, message logging, paste uploads, support
thread replies, moderation checks, AI-assisted answers, and local image OCR.

Bot definitions live in [`src/bots`](src/bots). Each bot has a typed Rust
configuration for its identity, commands, responses, prompts, checks, and
support links.

## Requirements

- Rust 1.97.1 or newer
- On Linux, OpenSSL development headers and `pkg-config` to build Steward
  (`libssl-dev` and `pkg-config` on Debian and Ubuntu)
- On Linux, the OpenSSL shared libraries and CA certificates to run Steward
- Discord applications with the Message Content privileged intent enabled
- A token for each configured bot
- OpenRouter and Brave Search API keys for AI support replies

## Configuration

Set these environment variables in the shell, a local `.env` file, or the
deployment environment:

| Variable | Purpose |
| --- | --- |
| `DISCORD_TOKEN_STEWARD` | Discord token for Steward |
| `DISCORD_TOKEN_JARVIS` | Discord token for Jarvis |
| `OPENROUTER_API_KEY` | OpenRouter access for support answers |
| `BRAVE_SEARCH_API_KEY` | Current web context for support answers |
| `RUST_LOG` | Optional log filter, such as `steward=debug,info` |

Steward checks both Discord tokens and both AI keys at startup and stops if one
is missing.

Both bots use [`deepseek/deepseek-v4.1-flash`](https://openrouter.ai/deepseek/deepseek-v4.1-flash)
through OpenRouter. To change the model, edit `AiConfig.model` in the matching
bot definition. Choose an OpenRouter model that supports tool calling for Brave Search.

## Features

Each bot definition in [`src/bots`](src/bots) sets which features run. A bot
acts only in the servers in its `guild_ids`. To turn off an optional feature,
set its field to `None` or to an empty list.

| Feature | What it does | How to turn it off |
| --- | --- | --- |
| Support commands | Registers the slash commands in `commands.responses`, `/help`, `/latest`, and `/resolved`. `/resolved` tags, locks, and archives a forum post. The Send Help and Send to Support Forum message commands answer a selected message. | Remove entries from `commands.responses`. Set `commands.latest` to `None` to remove `/latest`. |
| AI chat | Answers each message in channels whose name starts with an entry in `chatbot.channel_name_prefixes`. It uses OpenRouter, Brave Search, and the pages in `docs_context_urls` and `docs_page_prefix`. It removes links that do not match `allowed_links`. Moderators can also use the Reply with AI message command in any channel. | Set `channel_name_prefixes` to `&[]`. |
| Text checks | Reads paste links that match `checks.paste_checks` and the text in screenshots. Replies when the text contains a `checks.text_checks` entry. | Set both lists to `&[]`. |
| Dump analysis | Summarizes a pasted debug dump and compares its version with the release at `checks.release_url`. Only Steward has one. | Set `checks.dump_analyzer` to `None`. |
| Image spam removal | Deletes a message when the text in its image contains both "crypto" and "casino", and kicks the sender. Members that the honeypot exempts are not kicked. | Always on. |
| Paste uploads | Uploads text, JSON, YAML, and XML attachments in public channels to pastes.dev and replies with the link. It skips files with names such as `.env` or `token`. | Set `autoupload` to `None`. |
| Thread starter | Posts a support message in new posts in the `forum_ids` forums. | Set `thread_starter` to `None`. |
| Message replies | Replies to common mistakes, such as a server command typed in Discord. | Set `message_replies` to `&[]`. |
| No-ping warning | Warns members who ping a `staff_role_ids` role or member outside an active conversation. | Set `no_ping` to `None`. |
| Honeypot | Bans members who post in a trap channel. See [Honeypot channels](#honeypot-channels). | Set `honeypot` to `None`. |
| Message logs | Writes each member message to a daily JSON Lines file in `logs_dir`. | Always on. |

The bot needs Manage Messages and Kick Members for image spam removal, and
Manage Threads for `/resolved`.

## Honeypot channels

Both bots ban members who post in a text channel named exactly
`interesting-chat`. Spam scripts on compromised accounts often post in every
accessible channel. A honeypot catches these accounts without inspecting the
message content. See [Honeypot's explanation](https://honeypot.riskymh.dev/docs)
for more context.

1. Create a text channel named `interesting-chat` in the server.
2. Allow members to view the channel and send messages.
3. Disable public and private thread creation in the channel.
4. Add this warning to the channel topic and a pinned message:

   > Do not post here. This channel catches spam accounts. Posting here causes
   > an immediate ban, even if you post by accident.

5. Give the bot the View Channel and Ban Members permissions.
6. Place the bot role above the member roles that it must ban.

Any member message triggers the ban, including replies and messages with only
attachments. The ban removes the last 24 hours of that member's messages across
the server. The bot records the channel and message IDs in the Discord audit
log reason. It also logs successful bans and errors through tracing.

Bots, webhooks, the server owner, and configured staff roles are exempt.
Members with Administrator, Manage Server, Ban Members, Kick Members, or
Timeout Members permissions are also exempt. Discord prevents bans of members
whose highest role is equal to or above the bot's highest role.
See [Discord's permission hierarchy](https://docs.discord.com/developers/topics/permissions#permission-hierarchy)
and [ban API](https://docs.discord.com/developers/resources/guild#create-guild-ban).

Each bot definition contains a `honeypot` configuration. Change `channel_names`
to watch other exact names, or set `honeypot` to `None` to disable the feature.
The name applies to every server in the bot's `guild_ids`. Renaming a channel
disables the trap unless its new name also matches.
Use `exempt_role_ids` to protect additional roles. Set `delete_message_days`
between `0` and `7` to change the cleanup period.

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
cargo deny --locked check # Requires cargo-deny 0.20.2
cargo test --all-targets
cargo build --release --locked
```

## Deploy with Railpack

Railpack detects the root `Cargo.toml` as a Rust application, builds the
`steward` release binary, and starts it as `./bin/steward`. The repository does
not need a Railpack configuration file, Dockerfile, or Compose file.

Configure the environment variables above in the deployment service and deploy
the repository root. Railpack supplies OpenSSL headers and `pkg-config` in its
build image. Its runtime image supplies the OpenSSL libraries and CA certificates.

## Run the published image

Pushes to `main` build a release image with Railpack and publish it to
`ghcr.io/skinsrestorer/steward`. The workflow publishes three tags for both architectures:

- `latest`: the latest published build from `main`
- `main`: the latest published build from `main`
- `sha-<12-character-commit-sha>`: the build for a specific commit

The workflow builds amd64 on `ubuntu-26.04` and arm64 on `ubuntu-26.04-arm`.
It authenticates to GHCR with `GITHUB_TOKEN` and `packages: write` permission.

The workflow publishes only commits whose build, tests, and dependency checks
passed. After publishing, it calls the Dokploy deploy webhook. Set the
`DOKPLOY_WEBHOOK_URL` repository secret, or the deploy step fails after the
image is pushed.

Put the required environment variables in a local `.env` file. Then run:

```bash
docker run -d --name steward --restart unless-stopped \
  --env-file .env -v steward-logs:/app/logs \
  ghcr.io/skinsrestorer/steward:latest
```

Each bot writes the messages it sees to daily JSON Lines files in
`/app/logs/<bot>`. Mount a volume there to keep them when the container is
recreated.

The image includes the OCR models. No model volume is required.

## Add or change a bot

Edit the matching file in [`src/bots`](src/bots). Each bot only works in the
servers listed in its `guild_ids` and only posts AI links that match its
`allowed_links`. Startup checks every definition against Discord's limits.
Add a new definition to
[`src/bots/mod.rs`](src/bots/mod.rs) when introducing another bot. Commands are
registered from these static definitions at startup.

The bundled OCR model provenance and license are documented in
[`models/README.md`](models/README.md).

## License

Steward is licensed under the GNU General Public License v3.0 or later. See
[`LICENSE`](LICENSE).

Steward started as a fork of the LuckPerms Discord bot. That project's MIT
license notice is kept in [`LICENSE.txt`](LICENSE.txt) as its license requires.

## Contributing and support

Read [the contribution guide](CONTRIBUTING.md) for development and review.
Use [the support guide](SUPPORT.md) for questions and issue routing.
