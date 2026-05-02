import type { Client } from "discord.js";
import type { BotConfig } from "@/bot-config";

// noinspection JSUnusedGlobalSymbols
export default (client: Client, bot: BotConfig): void => {
  const config = bot.messageReplies;
  if (config == null) {
    return;
  }

  client.on("messageCreate", async (message) => {
    if (!message.inGuild() || message.author.bot) return;

    for (const rule of config.rules) {
      if (rule.matches(message)) {
        await message.reply(rule.reply(message));
      }
    }
  });
};
