import fs from "node:fs";
import path from "node:path";
import dateFormat from "dateformat";
import type { Client } from "discord.js";
import type { BotConfig } from "@/bot-config";

const getLogFileName = (logsDir: string, date: number): string =>
  path.join(logsDir, `${dateFormat(date, "yyyy-mm-dd")}.log`);
const getLogFileTime = (date: number): string =>
  dateFormat(date, "hh-MM-ss TT");

// noinspection JSUnusedGlobalSymbols
export default (client: Client, bot: BotConfig): void => {
  if (!fs.existsSync(bot.logsDir)) {
    fs.mkdirSync(bot.logsDir, { recursive: true });
  }

  client.on("messageCreate", (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      message.author.bot
    )
      return;

    const date = Date.now();
    const log = `${getLogFileTime(date)} [${message.channel.name}] ${message.author.tag}: ${message.content}\n`;
    const logPath = getLogFileName(bot.logsDir, date);

    fs.appendFile(logPath, log, (err) => {
      if (err != null) {
        console.log(err);
      }
    });
  });
};
