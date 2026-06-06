import type { ModelMessage } from "ai";
import type {
  AnyThreadChannel,
  Client,
  ColorResolvable,
  Message,
  MessageCreateOptions,
  MessageReplyOptions,
  PresenceData,
} from "discord.js";
import type { LatestReleaseResponse } from "@/lib/github-release";

export type BotModule = (
  client: Client,
  bot: BotConfig,
) => Promise<void> | void;

export interface BotConfig {
  id: string;
  name: string;
  tokenEnv: string;
  clientId: string;
  accentColor: ColorResolvable;
  logsDir: string;
  presence: PresenceData;
  modules: BotModule[];
  autoupload?: AutouploadConfig;
  chatbot?: ChatbotConfig;
  checks?: ChecksConfig;
  commands?: CommandsConfig;
  messageReplies?: MessageRepliesConfig;
  noPing?: NoPingConfig;
  threadStarterReply?: ThreadStarterReplyConfig;
}

export interface AutouploadConfig {
  userAgent: string;
  futureUploadsMessage: (attachmentName: string, uploadedUrl: string) => string;
  failedUploadMessage: string;
}

export interface SupportAiConfig {
  applicationGuardrailMessage: string;
  docsContextUrls?: string[];
  maxOutputTokens?: number;
  model?: string;
  promptInjectionPatterns: readonly RegExp[];
  responseDisclaimer?: string;
  systemPrompt: string;
  webSearch?: SupportAiWebSearchConfig;
}

export interface SupportAiWebSearchConfig {
  maxContextTokens?: number;
  provider: "brave";
}

export interface ChatbotConfig {
  ai: SupportAiConfig;
  channelNamePrefixes: string[];
  generationErrorMessage: string;
  maxResponseLength?: number;
  promptInjectionErrorMessage: string;
}

export interface CommandField {
  key: string;
  value: string;
}

export interface ConfigCommand {
  name: string;
  cmdDescription: string;
  url?: string;
  title: string;
  description: string;
  docs?: boolean;
  fields?: CommandField[];
}

export interface CommandsConfig {
  commandResponses: ConfigCommand[];
  docsFooter?: {
    iconURL?: string;
    text: string;
  };
  help?: {
    description: string;
    embedDescription: string;
    embedTitle: string;
  };
  latest?: {
    description: string;
    releaseUrl: string;
    title: string;
  };
  replyWithAiContext?: {
    ai: SupportAiConfig;
    name: string;
    requesterPrompt: (requesterMention: string, message: string) => string;
  };
  resolved?: {
    alreadyResolvedMessage: string;
    description: string;
    successMessage: string;
    tagId: string;
  };
  sendHelpContext?: {
    name: string;
  };
  sendSupportContext?: {
    embedDescription: string;
    embedTitle: string;
    embedUrl?: string;
    name: string;
  };
}

export interface Checks {
  regex: RegExp;
  getLink: string;
}

export type MessagePredicate = (message: string) => boolean;

export interface Tests {
  checks: (RegExp | MessagePredicate)[];
  title: string;
  content: string;
  tips?: string[];
  link?: string;
}

export interface DumpAnalysisContext {
  footer: string;
  latestRelease: LatestReleaseResponse | null;
  text: string;
}

export interface DumpAnalysisResult {
  content: string;
  embeds: MessageCreateOptions["embeds"];
  files?: MessageCreateOptions["files"];
}

export interface ChecksConfig {
  dumpAnalyzer?: (
    context: DumpAnalysisContext,
  ) => DumpAnalysisResult | null | Promise<DumpAnalysisResult | null>;
  pasteChecks: Checks[];
  releaseUrl?: string;
  tests: Tests[];
}

export interface MessageReplyRule {
  matches: (message: Message<true>) => boolean;
  reply: (message: Message<true>) => MessageReplyOptions;
}

export interface MessageRepliesConfig {
  rules: MessageReplyRule[];
}

export interface NoPingConfig {
  exemptRoleIds: string[];
  staffRoleIds: string[];
  warningMessage: (message: Message<true>) => string;
}

export interface ThreadStarterReplyConfig {
  buildReply: (
    thread: AnyThreadChannel,
  ) => MessageReplyOptions | null | Promise<MessageReplyOptions | null>;
}

export interface GenerateSupportResponseOptions {
  maxLength?: number;
}

export interface GenerateSupportResponseResult {
  responseMessages: ModelMessage[];
  text: string;
}
