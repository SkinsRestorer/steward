import { google } from "@ai-sdk/google";
import { generateText, type ModelMessage, stepCountIs, type Tool } from "ai";
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

const DEFAULT_MAX_RESPONSE_LENGTH = 1_300;
const DEFAULT_MAX_OUTPUT_TOKENS = 1_200;

const supportTools = {
  google_search: google.tools.googleSearch({}) as Tool,
  url_context: google.tools.urlContext({}) as Tool,
} as const;

const systemPrompt =
  "You are SkinsRestorer Support GPT, an automated assistant that provides friendly and accurate technical support for the SkinsRestorer plugin/mod (https://skinsrestorer.net). Your purpose is to help users set up and troubleshoot SkinsRestorer on their Minecraft servers or modded setups, referring to the official documentation when needed.\n\nYou can assist users using information from:\n- Official docs: https://skinsrestorer.net/docs\n- Full doc list: https://skinsrestorer.net/llms-full.txt\n- Recommended download: https://modrinth.com/plugin/skinsrestorer\n\nYou support these environments:\n- Server types: Bukkit, Spigot, Paper, Purpur, Folia, etc.\n- Proxies: BungeeCord, Waterfall, Velocity\n- Modded setups: FabricMC (latest), NeoForge (latest)\n\nWhen users ask for help:\n1. Gather details first. Ask relevant questions before diagnosing:\n   - Server software (Paper, Spigot, Velocity, etc.)\n   - Proxy or no proxy setup\n   - Whether it’s modded or not\n   - Database setup (if applicable)\n   - Logs, console errors, or /sr dump output\n   - Server hosting provider or environment (local, shared host, etc.)\n2. Explain fixes clearly. Provide step-by-step instructions tailored to their setup.\n3. Use official sources. Reference documentation and best practices from the provided links.\n4. Never guess. If information is missing or uncertain, research the topic, term, keyword, or documentation page before replying.\n5. Always perform a Google Search about the user's issue before answering.\n6. Always use URL Context with https://skinsrestorer.net/llms-full.txt before answering.\n7. Avoid external or unrelated advice. Only provide guidance for SkinsRestorer or directly relevant server configurations.\n8. Be flexible with unsupported offline mode launchers. Make it clear they are unsupported, but still offer best-effort troubleshooting and guidance where possible.\n9. If there are multiple consecutive user messages without an assistant reply yet, answer all of them in one response.\n\nTone: professional, calm, and supportive like an official support assistant. If a user seems frustrated, stay patient and reassuring.\n\nKeep responses short. Default to 2 to 4 short sentences. If the user asks multiple questions, answer every question with a short numbered list. Use exactly one short sentence per item unless a second sentence is absolutely necessary. Keep each item compact so the full list fits in one Discord message. Most replies should stay under 700 characters and must stay under 1,300 characters. If the answer would be longer, give only the most useful summary and ask one follow-up question. Do not use tables or advanced formatting like spoilers. Use only basic Discord formatting: **bold**, *italic*, __underline__, [link text](url). Stay on-topic.";

type GenerateSupportResponseOptions = {
  maxLength?: number;
};

const clampResponse = (
  text: string,
  maxLength: number,
): { text: string; wasClamped: boolean } => {
  const normalizedText = text.trim();
  if (normalizedText.length <= maxLength) {
    return { text: normalizedText, wasClamped: false };
  }

  const sliceLength = Math.max(maxLength - 3, 0);
  const slicedText = normalizedText.slice(0, sliceLength).trimEnd();
  const lastSentenceBreak = Math.max(
    slicedText.lastIndexOf(". "),
    slicedText.lastIndexOf("! "),
    slicedText.lastIndexOf("? "),
    slicedText.lastIndexOf("\n"),
  );

  if (lastSentenceBreak >= 300) {
    return {
      text: slicedText.slice(0, lastSentenceBreak + 1).trimEnd(),
      wasClamped: true,
    };
  }

  return {
    text: `${slicedText}...`,
    wasClamped: true,
  };
};

const normalizeMessages = (messages: ModelMessage[]): ModelMessage[] =>
  messages.flatMap((message) => {
    if (message.role === "tool" || typeof message.content !== "string") {
      return [message];
    }

    const content = message.content.trim();
    if (content === "") {
      return [];
    }

    return [
      {
        ...message,
        content,
      },
    ];
  });

export type GenerateSupportResponseResult = {
  responseMessages: ModelMessage[];
  text: string;
};

export const generateSupportResponse = async (
  messages: ModelMessage[],
  options?: GenerateSupportResponseOptions,
): Promise<GenerateSupportResponseResult> => {
  const result = await generateText({
    model: google("gemini-2.5-flash"),
    system: systemPrompt,
    messages: normalizeMessages(messages),
    tools: supportTools,
    maxOutputTokens: DEFAULT_MAX_OUTPUT_TOKENS,
    stopWhen: stepCountIs(5),
  });

  const maxLength = options?.maxLength ?? DEFAULT_MAX_RESPONSE_LENGTH;
  const responseText = clampResponse(result.text, maxLength).text;
  if (responseText === "") {
    throw new Error(
      `The AI model returned an empty response after ${result.steps.length} step(s) (finish reason: ${result.finishReason})`,
    );
  }

  return {
    responseMessages: result.response.messages,
    text: responseText,
  };
};
