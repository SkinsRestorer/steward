import type { Client, Message } from "discord.js";
import type { BotConfig } from "@/bot-config";

const minuteInMs = 60 * 1000;
const hourInMs = 60 * minuteInMs;
const dayInMs = 24 * hourInMs;
const discordEpochMs = 1_420_070_400_000;
const discordEpochMsBigInt = BigInt(discordEpochMs);
const replyConversationWindowMs = hourInMs;
const participationWindowMs = dayInMs;
const recentParticipationGraceMs = 5 * minuteInMs;

const createSnowflakeAfterTimestamp = (timestampMs: number): string => {
  const timestamp = Math.max(discordEpochMs, Math.floor(timestampMs) + 1);

  return ((BigInt(timestamp) - discordEpochMsBigInt) << 22n).toString();
};

const memberIsStaff = (
  member: Message<true>["member"],
  bot: BotConfig,
): boolean =>
  member?.roles.cache.some((role) =>
    bot.noPing?.staffRoleIds.includes(role.id),
  ) === true;

const mentionsStaff = (message: Message<true>, bot: BotConfig): boolean =>
  message.mentions.members.some((member) => memberIsStaff(member, bot)) ||
  message.mentions.roles.some((role) =>
    bot.noPing?.staffRoleIds.includes(role.id),
  );

const memberIsExemptFromNoPingRule = (
  member: Message<true>["member"],
  bot: BotConfig,
): boolean =>
  member?.roles.cache.some((role) =>
    bot.noPing?.exemptRoleIds.includes(role.id),
  ) === true;

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

const fetchReferencedMessage = async (
  message: Message<true>,
): Promise<Message<true> | null> => {
  try {
    return await message.fetchReference();
  } catch {
    return null;
  }
};

const replyIsPartOfActiveConversation = (
  message: Message<true>,
  reference: Message<true>,
): boolean =>
  message.createdTimestamp - reference.createdTimestamp <=
  replyConversationWindowMs;

const replyTargetWasReplyToSender = async (
  message: Message<true>,
  reference: Message<true>,
): Promise<boolean> => {
  if (reference.reference?.messageId == null) {
    return false;
  }

  const referencedReplyTarget = await fetchReferencedMessage(reference);

  return referencedReplyTarget?.author.id === message.author.id;
};

const shouldSkipReplyWarning = async (
  message: Message<true>,
): Promise<boolean> => {
  if (message.reference === null) {
    return false;
  }

  const reference = await fetchReferencedMessage(message);

  if (reference == null) {
    return false;
  }

  if (await replyTargetWasReplyToSender(message, reference)) {
    return true;
  }

  return (
    replyIsPartOfActiveConversation(message, reference) &&
    (await senderIsActiveChatParticipant(message))
  );
};

// noinspection JSUnusedGlobalSymbols
export default (client: Client, bot: BotConfig): void => {
  const config = bot.noPing;
  if (config == null) {
    return;
  }

  client.on("messageCreate", async (message) => {
    if (!message.inGuild() || message.author.bot) return;

    if (
      message.mentions.members.size === 0 &&
      message.mentions.roles.size === 0
    )
      return;

    if (memberIsStaff(message.member, bot)) {
      return;
    }

    if (memberIsExemptFromNoPingRule(message.member, bot)) {
      return;
    }

    if (!mentionsStaff(message, bot)) {
      return;
    }

    if (await shouldSkipReplyWarning(message)) {
      return;
    }

    // Tell them off:
    await message.reply(config.warningMessage(message));
  });
};
