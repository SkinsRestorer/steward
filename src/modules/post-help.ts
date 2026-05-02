import { ChannelType, type Client } from "discord.js";
import type { BotConfig } from "@/bot-config";

// noinspection JSUnusedGlobalSymbols
export default (client: Client, bot: BotConfig): void => {
  const config = bot.threadStarterReply;
  if (config == null) {
    return;
  }

  client.on("threadCreate", async (thread) => {
    if (thread.parent?.type !== ChannelType.GuildForum) return;
    if (!thread.joinable) return;

    await thread.join();

    const originalMessage = await thread.fetchStarterMessage();
    if (!originalMessage) return;

    const reply = await config.buildReply(thread);
    if (reply == null) {
      return;
    }

    await originalMessage.reply(reply);
  });
};
