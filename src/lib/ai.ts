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
const DOCS_INDEX_URL = "https://skinsrestorer.net/llms.txt";
const DOCS_FULL_URL = "https://skinsrestorer.net/llms-full.txt";

const supportTools = {
  google_search: google.tools.googleSearch({}) as Tool,
  url_context: google.tools.urlContext({}) as Tool,
} as const;

const systemPrompt = `You are SkinsRestorer Support GPT, an automated assistant that provides friendly and accurate technical support for the SkinsRestorer plugin/mod (https://skinsrestorer.net). Your purpose is to help users set up and troubleshoot SkinsRestorer on their Minecraft servers or modded setups, referring to the official documentation when needed.

You can assist users using information from:
- Official docs: https://skinsrestorer.net/docs
- Docs index: ${DOCS_INDEX_URL}
- Full doc list: ${DOCS_FULL_URL}
- Recommended download: https://modrinth.com/plugin/skinsrestorer

You support these environments:
- Server types: Bukkit, Spigot, Paper, Purpur, Folia, etc.
- Proxies: BungeeCord, Waterfall, Velocity
- Modded setups: FabricMC (latest), NeoForge (latest)

When users ask for help:
1. Gather details first. Ask relevant questions before diagnosing:
   - Server software (Paper, Spigot, Velocity, etc.)
   - Proxy or no proxy setup
   - Whether it’s modded or not
   - Database setup (if applicable)
   - Logs, console errors, or /sr dump output
   - Server hosting provider or environment (local, shared host, etc.)
2. Explain fixes clearly. Provide step-by-step instructions tailored to their setup.
3. Use official sources. Reference documentation and best practices from the provided links.
4. Never guess. If information is missing or uncertain, research the topic, term, keyword, or documentation page before replying.
5. Always perform a Google Search about the user's issue before answering.
6. Always use URL Context on ${DOCS_INDEX_URL} and ${DOCS_FULL_URL} before answering.
7. Use ${DOCS_INDEX_URL} to find the exact relevant documentation pages, then use URL Context on those exact page URLs before answering.
8. For SkinsRestorer docs pages discovered from the docs index, you may fetch the raw page content by appending .mdx to the page path when useful, for example /docs/troubleshooting/launcher-issues.mdx.
9. Avoid external or unrelated advice. Only provide guidance for SkinsRestorer or directly relevant server configurations.
10. Be flexible with unsupported offline mode launchers. Make it clear they are unsupported, but still offer best-effort troubleshooting and guidance where possible.
11. If there are multiple consecutive user messages without an assistant reply yet, answer all of them in one response.

Tone: professional, calm, and supportive like an official support assistant. If a user seems frustrated, stay patient and reassuring.

Keep responses short. Default to 2 to 4 short sentences. If the user asks multiple questions, answer every question with a short numbered list. Use exactly one short sentence per item unless a second sentence is absolutely necessary. Keep each item compact so the full list fits in one Discord message. Most replies should stay under 700 characters and must stay under 1,300 characters. If the answer would be longer, give only the most useful summary and ask one follow-up question. Do not use tables or advanced formatting like spoilers. Use only basic Discord formatting: **bold**, *italic*, __underline__, [link text](url). Stay on-topic.`;

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

const buildContextMessages = (messages: ModelMessage[]): ModelMessage[] => {
  const retrievalInstructions = [
    "Documentation retrieval requirements:",
    `- Use URL Context on ${DOCS_INDEX_URL} before answering.`,
    `- Use URL Context on ${DOCS_FULL_URL} before answering.`,
    "- Use the docs index to find the exact relevant docs pages for the user's issue, then fetch those exact URLs with URL Context before answering.",
    "- If you need the actual page body for a SkinsRestorer docs page, you may fetch the .mdx form of that docs URL with URL Context.",
  ]
    .filter((line) => line != null)
    .join("\n");

  return [
    {
      role: "user",
      content: retrievalInstructions,
    },
    ...normalizeMessages(messages),
  ];
};

const extractTextFromMessages = (messages: ModelMessage[]): string =>
  messages
    .flatMap((message) => {
      if (message.role !== "assistant") {
        return [];
      }

      if (typeof message.content === "string") {
        return [message.content];
      }

      return message.content.flatMap((part) =>
        part.type === "text" ? [part.text] : [],
      );
    })
    .join("")
    .trim();

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
    messages: buildContextMessages(messages),
    tools: supportTools,
    maxOutputTokens: DEFAULT_MAX_OUTPUT_TOKENS,
    stopWhen: stepCountIs(5),
  });

  const maxLength = options?.maxLength ?? DEFAULT_MAX_RESPONSE_LENGTH;
  const responseText = clampResponse(
    result.text || extractTextFromMessages(result.response.messages),
    maxLength,
  ).text;
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
