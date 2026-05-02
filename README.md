# steward
Our support discord bot! Most code taken from LuckPerms clippy. :D

## Bot configs

Bot identities live in `src/bots/*.ts`. Each config sets the token env var,
client ID, presence, modules, prompts, command copy, no-ping rules, and log
directory for one Discord bot.

Configured bots:

- Steward: `DISCORD_TOKEN_STEWARD`
- Jarvis: `DISCORD_TOKEN_JARVIS`

Client IDs are not secret and are stored directly in each bot config. To run
another bot in the same process, add another config file and import it in
`src/bots/index.ts`.
