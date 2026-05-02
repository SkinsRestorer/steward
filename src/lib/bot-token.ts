import type { BotConfig } from "@/bot-config";

export const getBotToken = (bot: BotConfig): string => {
  const token =
    process.env[bot.tokenEnv] ??
    (bot.fallbackTokenEnv == null
      ? undefined
      : process.env[bot.fallbackTokenEnv]);

  if (token == null || token === "") {
    const fallbackText =
      bot.fallbackTokenEnv == null ? "" : ` or ${bot.fallbackTokenEnv}`;

    throw new Error(
      `${bot.name} token must be provided via ${bot.tokenEnv}${fallbackText}`,
    );
  }

  return token;
};
