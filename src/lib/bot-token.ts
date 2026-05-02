import type { BotConfig } from "@/bot-config";

export const getBotToken = (bot: BotConfig): string => {
  const token = process.env[bot.tokenEnv];

  if (token == null || token === "") {
    throw new Error(`${bot.name} token must be provided via ${bot.tokenEnv}`);
  }

  return token;
};
