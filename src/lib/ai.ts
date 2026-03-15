import { google } from "@ai-sdk/google";
import { generateText, type Tool } from "ai";
import "dotenv/config";

const googleApiKey =
  process.env.GEMINI_API_KEY ??
  process.env.GOOGLE_GENERATIVE_AI_API_KEY ??
  process.env.GOOGLE_API_KEY;

if (googleApiKey == null) {
  throw new Error(
    "A Gemini API key must be provided via GEMINI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY, or GOOGLE_API_KEY",
  );
}

process.env.GOOGLE_GENERATIVE_AI_API_KEY ??= googleApiKey;

const supportTools = {
  google_search: google.tools.googleSearch({}) as Tool,
  url_context: google.tools.urlContext({}) as Tool,
} as const;

const systemPrompt =
  "You are SkinsRestorer Support GPT, an automated assistant that provides friendly and accurate technical support for the SkinsRestorer plugin/mod (https://skinsrestorer.net). Your purpose is to help users set up and troubleshoot SkinsRestorer on their Minecraft servers or modded setups, referring to the official documentation when needed.\n\nYou can assist users using information from:\n- Official docs: https://skinsrestorer.net/docs\n- Full doc list: https://skinsrestorer.net/llms-full.txt\n- Recommended download: https://modrinth.com/plugin/skinsrestorer\n\nYou support these environments:\n- Server types: Bukkit, Spigot, Paper, Purpur, Folia, etc.\n- Proxies: BungeeCord, Waterfall, Velocity\n- Modded setups: FabricMC (latest), NeoForge (latest)\n\nWhen users ask for help:\n1. Gather details first. Ask relevant questions before diagnosing:\n   - Server software (Paper, Spigot, Velocity, etc.)\n   - Proxy or no proxy setup\n   - Whether it’s modded or not\n   - Database setup (if applicable)\n   - Logs, console errors, or /sr dump output\n   - Server hosting provider or environment (local, shared host, etc.)\n2. Explain fixes clearly. Provide step-by-step instructions tailored to their setup.\n3. Use official sources. Reference documentation and best practices from the provided links.\n4. Never guess. If information is missing or uncertain, research the topic, term, keyword, or documentation page before replying.\n5. Always perform a web search about the user's issue before answering. No exceptions.\n6. Avoid external or unrelated advice. Only provide guidance for SkinsRestorer or directly relevant server configurations.\n7. Be flexible with unsupported offline mode launchers. Make it clear they are unsupported, but still offer best-effort troubleshooting and guidance where possible.\n8. Always do a web search before answering. Search especially for the most complex topics and documentation pages before giving any answer.\n\nTone: professional, calm, and supportive like an official support assistant. If a user seems frustrated, stay patient and reassuring.\n\nKeep responses concise because Discord has a 2000 character limit. Do not use tables or advanced formatting like spoilers. Use only basic Discord formatting: **bold**, *italic*, __underline__, [link text](url). Stay on-topic.";

export type SupportChatMessage = {
  role: "user" | "assistant";
  content: string;
};

type GenerateSupportResponseOptions = {
  maxLength?: number;
};

const buildPrompt = (messages: SupportChatMessage[]): string => {
  const conversation = messages
    .map(({ role, content }) => {
      const label = role === "user" ? "User" : "Assistant";
      return `${label}: ${content.trim()}`;
    })
    .join("\n\n");

  return [
    "Always run Google Search before answering.",
    "Always use URL Context with https://skinsrestorer.net/llms-full.txt before answering.",
    "Prefer official SkinsRestorer documentation and the Modrinth download page when relevant.",
    "",
    "Reference URLs:",
    "- https://skinsrestorer.net/llms-full.txt",
    "- https://skinsrestorer.net/docs",
    "- https://modrinth.com/plugin/skinsrestorer",
    "",
    "Conversation:",
    conversation,
    "",
    "Answer the latest user message.",
  ].join("\n");
};

export const generateSupportResponse = async (
  messages: SupportChatMessage[],
  options?: GenerateSupportResponseOptions,
): Promise<string> => {
  const { text } = await generateText({
    model: google("gemini-2.5-flash"),
    system: systemPrompt,
    prompt: buildPrompt(messages),
    tools: supportTools,
    maxOutputTokens: Math.round(1_750 / 4),
  });

  const maxLength = options?.maxLength ?? 2_000;
  if (text.length > maxLength) {
    const sliceLength = Math.max(maxLength - 3, 0);
    return `${text.slice(0, sliceLength)}...`;
  }

  return text;
};
