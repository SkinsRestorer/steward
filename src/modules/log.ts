import fs from "node:fs";
import dateFormat from "dateformat";
import type { Client } from "discord.js";

const getLogFileName = (date: number): string =>
  `logs/${dateFormat(date, "yyyy-mm-dd")}.log`;
const getLogFileTime = (date: number): string =>
  dateFormat(date, "hh-MM-ss TT");

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  if (!fs.existsSync("logs")) fs.mkdirSync("logs");

  client.on("messageCreate", (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      message.author.bot
    )
      return;

    const date = Date.now();
    const log = `${getLogFileTime(date)} [${message.channel.name}] ${message.author.tag}: ${message.content}\n`;
    const path = getLogFileName(date);

    fs.appendFile(path, log, (err) => {
      if (err != null) {
        console.log(err);
      }
    });
  });
};
