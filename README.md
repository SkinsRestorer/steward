# steward
Our support discord bot! Most code taken from LuckPerms clippy. :D

## Bot configs

Bot identities live in `src/bots/*.ts`. Each config sets the token env var,
client ID, presence, modules, prompts, command copy, no-ping rules, and log
directory for one Discord bot.

Steward reads `DISCORD_TOKEN_STEWARD` first and falls back to `DISCORD_TOKEN`.
To run another bot in the same process, add another config file, import it in
`src/bots/index.ts`, and give it its own token env var such as
`DISCORD_TOKEN_JARVIS`.
