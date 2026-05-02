import type { Client } from "discord.js";
import data from "@/data.json" with { type: "json" };

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on("messageCreate", async (message) => {
    if (
      !message.channel.isTextBased() ||
      message.channel.isDMBased() ||
      message.author.bot
    )
      return;

    if (message.mentions.members?.size === 0) return;
    if (message.reference !== null) return;

    const senderIsStaff = message.member?.roles.cache.some((role) =>
      data.staff_roles.includes(role.name),
    );
    if (senderIsStaff === true) {
      return;
    }

    const senderIsBot = message.author.bot;
    if (senderIsBot) {
      return;
    }

    const mentionsStaff = message.mentions.members?.some((member) => {
      // If the message mentions any members that satisfy the following:
      return member.roles.cache.some((role) =>
        data.staff_roles.includes(role.name),
      );
    });

    if (mentionsStaff === true) {
      // Tell them off:
      await message.reply(
        `Hi ${message.member?.nickname ?? message.author.username}! Free public support is currently not very fast because we can't afford doing free support 24/7 because we have other projects to work on and other responsibilities IRL. If this matter is important to you and you want to receive priority & private support, go to <#1314315764253200394> or https://skinsrestorer.net/pricing

-# If your message was not about support or a feature request, ignore this message.`,
      );
    }
  });
};
