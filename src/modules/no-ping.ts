import type { Client, Message } from "discord.js";
import data from "@/data.json" with { type: "json" };

const minuteInMs = 60 * 1000;
const hourInMs = 60 * minuteInMs;
const dayInMs = 24 * hourInMs;
const discordEpochMs = 1_420_070_400_000;
const discordEpochMsBigInt = BigInt(discordEpochMs);
const replyConversationWindowMs = hourInMs;
const participationWindowMs = dayInMs;
const recentParticipationGraceMs = 5 * minuteInMs;
const noPingExemptRoleIds = new Set(["1492530262993801457"]);

const createSnowflakeAfterTimestamp = (timestampMs: number): string => {
  const timestamp = Math.max(discordEpochMs, Math.floor(timestampMs) + 1);

  return ((BigInt(timestamp) - discordEpochMsBigInt) << 22n).toString();
};

const memberIsStaff = (member: Message<true>["member"]): boolean =>
  member?.roles.cache.some((role) => data.staff_roles.includes(role.name)) ===
  true;

const mentionsStaff = (message: Message<true>): boolean =>
  message.mentions.members.some((member) => memberIsStaff(member));

const memberIsExemptFromNoPingRule = (
  member: Message<true>["member"],
): boolean =>
  member?.roles.cache.some((role) => noPingExemptRoleIds.has(role.id)) === true;

const hasParticipatedBeforeReplyPing = async (
  message: Message<true>,
  before: string,
  earliestTimestamp: number,
): Promise<boolean> => {
  const messages = await message.channel.messages.fetch({
    limit: 100,
    before,
    cache: false,
  });

  if (messages.size === 0) {
    return false;
  }

  if (
    messages.some(
      (candidate) =>
        candidate.author.id === message.author.id &&
        candidate.createdTimestamp >= earliestTimestamp,
    )
  ) {
    return true;
  }

  const oldestMessage = messages.reduce((oldest, candidate) =>
    candidate.createdTimestamp < oldest.createdTimestamp ? candidate : oldest,
  );

  if (oldestMessage.createdTimestamp < earliestTimestamp) {
    return false;
  }

  return hasParticipatedBeforeReplyPing(
    message,
    oldestMessage.id,
    earliestTimestamp,
  );
};

const senderIsActiveChatParticipant = async (
  message: Message<true>,
): Promise<boolean> => {
  const earliestTimestamp = message.createdTimestamp - participationWindowMs;
  const latestTimestamp = message.createdTimestamp - recentParticipationGraceMs;
  const latestSnowflake = createSnowflakeAfterTimestamp(latestTimestamp);

  try {
    return await hasParticipatedBeforeReplyPing(
      message,
      latestSnowflake,
      earliestTimestamp,
    );
  } catch {
    return false;
  }
};

const replyIsPartOfActiveConversation = async (
  message: Message<true>,
): Promise<boolean> => {
  try {
    const reference = await message.fetchReference();

    return (
      message.createdTimestamp - reference.createdTimestamp <=
      replyConversationWindowMs
    );
  } catch {
    return false;
  }
};

const shouldSkipReplyWarning = async (
  message: Message<true>,
): Promise<boolean> =>
  message.reference !== null &&
  (await replyIsPartOfActiveConversation(message)) &&
  (await senderIsActiveChatParticipant(message));

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on("messageCreate", async (message) => {
    if (!message.inGuild() || message.author.bot) return;

    if (message.mentions.members.size === 0) return;

    if (memberIsStaff(message.member)) {
      return;
    }

    if (memberIsExemptFromNoPingRule(message.member)) {
      return;
    }

    if (!mentionsStaff(message)) {
      return;
    }

    if (await shouldSkipReplyWarning(message)) {
      return;
    }

    // Tell them off:
    await message.reply(
      `Hi ${message.member?.nickname ?? message.author.username}! Free public support is currently not very fast because we can't afford doing free support 24/7 because we have other projects to work on and other responsibilities IRL. If this matter is important to you and you want to receive priority & private support, go to <#1314315764253200394> or https://skinsrestorer.net/pricing

-# If your message was not about support or a feature request, ignore this message.`,
    );
  });
};
