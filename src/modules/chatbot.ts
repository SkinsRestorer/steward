import { groq } from "@ai-sdk/groq";
import {
  extractReasoningMiddleware,
  generateText,
  wrapLanguageModel,
} from "ai";
import type { Client, Message, Snowflake } from "discord.js";

type ChatMessage = {
  role: "user" | "assistant";
  content: string;
};

type Context = {
  isGenerating: () => boolean;
  messages: ChatMessage[];
  debounce: (message: Message) => void;
};
const userContext: Record<Snowflake, Context> = {};

const model = wrapLanguageModel({
  model: groq("openai/gpt-oss-120b"),
  middleware: extractReasoningMiddleware({ tagName: "think" }),
});

let systemPrompt = "Loading documentation...";

const fetchSystemPrompt = async (): Promise<void> => {
  try {
    const response = await fetch("https://skinsrestorer.net/llms.txt");
    systemPrompt = await response.text();
  } catch (error) {
    console.error("Failed to fetch system prompt:", error);
    systemPrompt = "Error loading documentation.";
  }
};

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  await fetchSystemPrompt();
  setInterval(fetchSystemPrompt, 24 * 60 * 60 * 1000); // Refresh every 24 hours
  client.on("messageCreate", async (message) => {
    const channel = message.channel;
    if (!channel.isTextBased() || channel.isDMBased() || message.author.bot)
      return;

    if (!channel.name.startsWith("chat-experiment")) return;

    const messageContent = message.content;
    let context = userContext[message.author.id];
    if (context != null) {
      const lastMessage = context.messages.at(-1);
      if (lastMessage?.role === "user") {
        lastMessage.content += `\n${messageContent}`;
      } else {
        context.messages.push({
          role: "user",
          content: messageContent,
        });
      }
    } else {
      let generating = false;
      setInterval(() => {
        if (generating) {
          void channel.sendTyping();
        }
      }, 8_000);

      const newContext: Context = {
        isGenerating: () => generating,
        messages: [
          {
            role: "user",
            content: messageContent,
          },
        ],
        debounce: debounce(async (message: Message) => {
          try {
            generating = true;
            await channel.sendTyping();
            const { text } = await generateText({
              model,
              messages: [
                {
                  role: "system",
                  content: systemPrompt,
                },
                {
                  role: "user",
                  content:
                    "Hi Steward! I have an issue with SkinsRestorer. Can you help me?",
                },
                {
                  role: "assistant",
                  content:
                    "Hello! Can you describe your issue? I wanna help you.",
                },
                ...newContext.messages,
              ],
              tools: {
                browser_search: groq.tools.browserSearch({}),
              } as {},
              maxOutputTokens: Math.round(1_750 / 4),
            });
            generating = false;

            let responseText = text;
            if (responseText.length > 2000) {
              responseText = `${responseText.slice(0, 2000 - 3)}...`;
            }

            newContext.messages.push({
              role: "assistant",
              content: responseText,
            });

            await message.reply(responseText);
          } catch (e) {
            generating = false;
            console.error(e);
          }
        }, 1_000),
      };

      context = userContext[message.author.id] = newContext;
    }

    if (context.isGenerating()) {
      await message.reply(
        "I'm still generating a response for your previous message. Please wait a moment.",
      );
    } else {
      context.debounce(message);
    }
  });
};

function debounce<TArgs extends unknown[]>(
  fn: (...args: TArgs) => unknown,
  delay: number,
): (...args: TArgs) => void {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  return (...args: TArgs): void => {
    // Clear the previous timeout
    if (timeoutId != null) {
      clearTimeout(timeoutId);
    }

    // Set a new timeout
    timeoutId = setTimeout(() => {
      void fn(...args);
    }, delay);
  };
}
