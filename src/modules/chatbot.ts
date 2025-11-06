import { groq } from "@ai-sdk/groq";
import { Search } from "@upstash/search";
import { generateId, generateText, stepCountIs, tool } from "ai";
import { z } from "zod";
import "dotenv/config";
import type { Client, Message, Snowflake } from "discord.js";

type ChatMessage = {
  role: "user" | "assistant";
  content: string;
};

const search = new Search({
  url: process.env.UPSTASH_SEARCH_REST_URL!,
  token: process.env.UPSTASH_SEARCH_REST_TOKEN!,
});

type KnowledgeContent = {
  text: string;
  section: string;
  title?: string;
};

const index = search.index<KnowledgeContent>("knowledge-base");

type Context = {
  isGenerating: () => boolean;
  messages: ChatMessage[];
  debounce: (message: Message) => void;
};
const userContext: Record<Snowflake, Context> = {};

const systemPrompt =
  "You are a support bot for SkinsRestorer. Use the searchKnowledge tool to find relevant information from the knowledge base to answer user questions. Keep responses very short due to Discord's 2000 character limit. Do not use table syntax or advanced formatting like spoilers. Use only basic Discord formatting: **bold**, *italic*, __underline__, [link text](url).\n\nYour task is to provide support to users that seek help with the plugin.\nUse short sentence since the user may not know Minecraft well, no yapping.\nYou are allowed to use Markdown format, but not other formats.\nAlways be on-topic, do not let the user go off-topic.";

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
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
              model: groq("openai/gpt-oss-120b"),
              stopWhen: stepCountIs(5),
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
                addResource: tool({
                  description:
                    "Add a new resource or piece of information to the knowledge base",
                  inputSchema: z.object({
                    resource: z
                      .string()
                      .describe(
                        "The content or resource to add to the knowledge base",
                      ),
                    title: z
                      .string()
                      .optional()
                      .describe("Optional title for the resource"),
                  }),
                  execute: async ({ resource, title }) => {
                    const id = generateId();
                    await index.upsert({
                      id,
                      content: {
                        text: resource,
                        section: "user-added",
                        title: title || `Resource ${id.slice(0, 8)}`,
                      },
                    });
                    return `Successfully added resource "${title || "Untitled"}" to knowledge base with ID: ${id}`;
                  },
                }),
                searchKnowledge: tool({
                  description:
                    "Search the knowledge base to find relevant information for answering questions",
                  inputSchema: z.object({
                    query: z
                      .string()
                      .describe(
                        "The search query to find relevant information",
                      ),
                    limit: z
                      .number()
                      .optional()
                      .describe(
                        "Maximum number of results to return (default: 3)",
                      ),
                  }),
                  execute: async ({ query, limit = 3 }) => {
                    const results = await index.search({
                      query,
                      limit,
                      reranking: true,
                    });

                    if (results.length === 0) {
                      return "No relevant information found in the knowledge base.";
                    }

                    return results.map((hit, i) => ({
                      resourceId: hit.id,
                      rank: i + 1,
                      title: hit.content.title || "Untitled",
                      content: hit.content.text || "",
                      section: hit.content.section || "unknown",
                      score: hit.score,
                    }));
                  },
                }),
                deleteResource: tool({
                  description: "Delete a resource from the knowledge base",
                  inputSchema: z.object({
                    resourceId: z
                      .string()
                      .describe("The ID of the resource to delete"),
                  }),
                  execute: async ({ resourceId }) => {
                    try {
                      await index.delete({ ids: [resourceId] });
                      return `Successfully deleted resource with ID: ${resourceId}`;
                    } catch (error) {
                      return `Failed to delete resource: ${error instanceof Error ? error.message : "Unknown error"}`;
                    }
                  },
                }),
              },
              onStepFinish: ({ toolResults }) => {
                if (toolResults.length > 0) {
                  console.log("Tool results:");
                  console.dir(toolResults, { depth: null });
                }
              },
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
