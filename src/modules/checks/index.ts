import { type Client, EmbedBuilder, type Message } from "discord.js";
import tesseract from "tesseract.js";
import type {
  BotConfig,
  Checks,
  ChecksConfig,
  MessagePredicate,
} from "@/bot-config";
import {
  getLatestReleaseProvider,
  type LatestReleaseResponse,
} from "@/lib/github-release";

const imageTypes = ["image/png", "image/jpeg", "image/webp"];

function findCheckMath(message: Message, config: ChecksConfig) {
  function matchToReturn(check: Checks, match: RegExpMatchArray) {
    if (!match[1]) {
      return null;
    }

    return {
      getLink: check.getLink.replace("{code}", match[1]),
      originalLink: match[0],
    };
  }

  for (const check of config.pasteChecks) {
    const match = check.regex.exec(message.content);
    if (match != null) {
      return matchToReturn(check, match);
    }

    for (const embedValue of message.embeds) {
      for (const field of embedValue.fields) {
        const match = check.regex.exec(field.value);
        if (match != null) {
          return matchToReturn(check, match);
        }
      }
    }
  }

  return null;
}

// noinspection JSUnusedGlobalSymbols
export default (client: Client, bot: BotConfig): void => {
  const config = bot.checks;
  if (config == null) {
    return;
  }

  const latestReleaseProvider =
    config.releaseUrl == null
      ? () => null
      : getLatestReleaseProvider(config.releaseUrl);

  client.on("messageCreate", async (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      message.author.bot
    )
      return;

    const checkResult = findCheckMath(message, config);
    if (!checkResult) {
      return;
    }

    try {
      console.log(`Getting upload bin ${checkResult.getLink}`);
      const response = await (await fetch(checkResult.getLink)).text();
      if (!response) {
        return;
      }

      await respondToText(
        bot,
        message,
        response,
        `${checkResult.originalLink} | Sent by ${message.author.username}`,
        latestReleaseProvider(),
      );
    } catch (error: unknown) {
      if (typeof error === "object" && error != null && "response" in error) {
        const status = (error as { response?: { status?: number } }).response
          ?.status;
        if (status === 404) {
          await message.reply({
            embeds: [
              new EmbedBuilder()
                .setTitle("Invalid Paste!")
                .setColor("#FF0000")
                .setDescription(
                  "The paste link you sent in is invalid or expired, please check the link or paste a new one.",
                )
                .setFooter({
                  text: `${checkResult.originalLink} | Sent by ${message.author.username}`,
                }),
            ],
          });
        }
      }
    }
  });

  client.on("messageCreate", async (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      message.author.bot
    )
      return;

    if (message.attachments.size === 0) {
      return;
    }

    const attachments = message.attachments
      .filter((attachment) => attachment.contentType != null)
      .filter((attachment) =>
        imageTypes.includes(attachment.contentType as string),
      );

    if (attachments.size === 0) {
      return;
    }

    for (const attachment of attachments.values()) {
      const {
        data: { text },
      } = await tesseract.recognize(attachment.proxyURL, "eng");

      await respondToText(
        bot,
        message,
        text,
        `Sent by ${message.author.username}`,
        latestReleaseProvider(),
      );
    }

    await message.react("👀");
  });
};

function checkMatch(text: string, checks: (RegExp | MessagePredicate)[]) {
  for (const check of checks) {
    if (typeof check === "function") {
      if (check(text)) {
        return text;
      }
    } else {
      const match = check.exec(text);
      if (match != null) {
        return match;
      }
    }
  }

  return null;
}

async function respondToText(
  bot: BotConfig,
  message: Message,
  text: string,
  footer: string,
  latestRelease: LatestReleaseResponse | null,
) {
  const config = bot.checks;
  if (config == null) {
    return;
  }

  for (const test of config.tests) {
    const cause = checkMatch(text, test.checks);
    if (cause == null) {
      continue;
    }

    const embed = new EmbedBuilder();
    embed.setTitle(test.title);
    embed.setDescription(test.content);
    if (test.tips != null) {
      embed.addFields(
        test.tips.map((tip, i) => ({ name: `Tip #${i + 1}`, value: tip })),
      );
    }

    if (test.link) {
      embed.addFields({ name: "Read More", value: test.link });
    }

    embed.addFields({ name: "Caused By", value: `\`\`\`${cause}\`\`\`` });
    embed.setFooter({ text: footer });
    embed.setColor(bot.accentColor);
    await message.reply({ embeds: [embed] });
  }

  if (config.dumpAnalyzer == null || !isJson(text)) {
    return;
  }

  try {
    const result = await config.dumpAnalyzer({ footer, latestRelease, text });
    if (result != null && result.embeds != null && result.embeds.length > 0) {
      await message.reply(result);
    }
  } catch (e) {
    console.error(e);
  }
}

function isJson(str: string) {
  try {
    JSON.parse(str);
    return true;
  } catch (_e) {
    return false;
  }
}
