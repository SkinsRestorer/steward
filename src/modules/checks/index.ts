import {
  type Client,
  type ColorResolvable,
  Colors,
  EmbedBuilder,
  type Message,
} from "discord.js";
import semver from "semver/preload";
import tesseract from "tesseract.js";
import data from "@/data.json" with { type: "json" };
import { getMetadata } from "../commands/metadata";
import config, { type Checks, type MessagePredicate } from "./checks-config";

const imageTypes = ["image/png", "image/jpeg", "image/webp"];

function findCheckMath(message: Message) {
  function matchToReturn(check: Checks, match: RegExpMatchArray) {
    if (!match[1]) {
      return null;
    }

    return {
      getLink: check.getLink.replace("{code}", match[1]),
      originalLink: match[0],
    };
  }

  for (const check of config.checks) {
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
export default (client: Client): void => {
  client.on("messageCreate", async (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      client.user?.id === message.author.id
    )
      return;

    const checkResult = findCheckMath(message);
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
        message,
        response,
        `${checkResult.originalLink} | Sent by ${message.author.username}`,
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
      client.user?.id === message.author.id
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

      await respondToText(message, text, `Sent by ${message.author.username}`);
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

async function respondToText(message: Message, text: string, footer: string) {
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
    embed.setColor(data.accent_color as ColorResolvable);
    await message.reply({ embeds: [embed] });
  }

  // Try parsing SkinsRestorer dump
  if (isJson(text)) {
    try {
      const rawDump = JSON.parse(text);

      const messageEmbeds: EmbedBuilder[] = [];
      {
        const { buildInfo } = rawDump;
        const version = semver.coerce(buildInfo.version);
        const metadata = getMetadata();
        const latestVersion = semver.coerce(metadata.tag_name);

        if (
          version !== null &&
          latestVersion !== null &&
          semver.lt(version, latestVersion)
        ) {
          messageEmbeds.push(
            new EmbedBuilder()
              .setTitle("Important: Outdated SkinsRestorer Version!")
              .setColor(Colors.Red)
              .setDescription(
                `The SkinsRestorer version you're using (\`${version}\`) is outdated! Please update to the latest version: \`${latestVersion}\``,
              )
              .addFields({
                name: " ",
                value: `[Download ${latestVersion}](https://modrinth.com/plugin/skinsrestorer/version/${latestVersion})`,
              }),
          );
        }

        messageEmbeds.push(
          new EmbedBuilder()
            .setTitle("Info: Build")
            .setColor(Colors.Blurple)
            .setDescription(
              `You are running version \`${version}\` of SkinsRestorer, built on \`${buildInfo.buildTime}\`.`,
            ),
        );
      }

      {
        const { osInfo, javaInfo, userInfo } = rawDump;
        if (
          userInfo.name === "?" ||
          userInfo.dir === "/home/container" ||
          userInfo.home === "/home/container"
        ) {
          messageEmbeds.push(
            new EmbedBuilder()
              .setTitle("Info: Docker detected")
              .setColor(Colors.Blurple)
              .setDescription(
                "We detected you are running SkinsRestorer in a Docker container and likely using a panel like Pterodactyl/Pelican. " +
                  "This is not an error, but we need to know this to better help you.",
              ),
          );
        }

        messageEmbeds.push(
          new EmbedBuilder()
            .setTitle("Info: OS/Java")
            .setColor(Colors.Blurple)
            .setDescription(
              `We detected you are running SkinsRestorer on \`${osInfo.name}\` with arch \`${osInfo.arch}\` and Java \`${javaInfo.version}\``,
            ),
        );
      }

      {
        const { platformInfo, environmentInfo } = rawDump;
        messageEmbeds.push(
          new EmbedBuilder()
            .setTitle("Info: Platform/Environment")
            .setColor(Colors.Blurple)
            .setDescription(
              `The dump is from the platform \`${platformInfo.platformName}\` (\`${environmentInfo.platform}\` & \`${environmentInfo.platformType}\`) with version \`${platformInfo.platformVersion}\` and \`${platformInfo.plugins.length}\` plugins.`,
            ),
        );
        if (environmentInfo.hybrid) {
          messageEmbeds.push(
            new EmbedBuilder()
              .setTitle("Warning: Hybrid detected!")
              .setColor(Colors.Red)
              .setDescription(
                `The platform appears to be a hybrid platform (mix of mods with plugins). This is not supported and may cause issues.`,
              ),
          );
        }
      }

      {
        const { pluginInfo } = rawDump;
        const { configData } = pluginInfo;
        messageEmbeds.push(
          new EmbedBuilder()
            .setTitle("Info: Plugin")
            .setColor(Colors.Blurple)
            .setDescription(
              `You are in proxy mode: \`${Boolean(pluginInfo.proxyMode)}\`, debug enabled: \`${Boolean(configData.dev.debug)}\``,
            ),
        );
      }

      if (messageEmbeds.length > 0) {
        await message.reply({
          content: `Found the following for: \`${footer}\``,
          embeds: messageEmbeds,
          files: [
            {
              contentType: "application/json",
              name: "dump.json",
              attachment: Buffer.from(JSON.stringify(rawDump, null, 2)),
            },
          ],
        });
      }
    } catch (e) {
      // Can be ignored, as it's not a SkinsRestorer dump
      console.error(e);
    }
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
