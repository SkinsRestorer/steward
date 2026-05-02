import { Client, GatewayIntentBits } from "discord.js";
import "dotenv/config";
import type { BotConfig } from "@/bot-config";
import { botConfigs } from "@/bots";
import { getBotToken } from "@/lib/bot-token";

const createClient = (bot: BotConfig): Client =>
  new Client({
    intents: [
      GatewayIntentBits.Guilds,
      GatewayIntentBits.GuildMembers,
      GatewayIntentBits.GuildPresences,
      GatewayIntentBits.GuildMessages,
      GatewayIntentBits.GuildMessageReactions,
      GatewayIntentBits.GuildMessageTyping,
      GatewayIntentBits.MessageContent,
    ],
    presence: bot.presence,
  });

const startBot = async (bot: BotConfig): Promise<void> => {
  const client = createClient(bot);

  client.on("ready", () => {
    console.log(`[${bot.id}] Logged in as ${client.user?.tag ?? "unknown"}!`);
  });

  for (const module of bot.modules) {
    console.log(`[${bot.id}] Loading module: ${module.name}`);
    await module(client, bot);
  }

  await client.login(getBotToken(bot));
};

await Promise.all(botConfigs.map(startBot));
