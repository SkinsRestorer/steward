import { generateSupportResponse, type SupportChatMessage } from "@/lib/ai";
import type { Client, Message, Snowflake } from "discord.js";

type Context = {
  isGenerating: () => boolean;
  messages: SupportChatMessage[];
  debounce: (message: Message) => void;
};
const userContext: Record<Snowflake, Context> = {};

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
            const responseText = await generateSupportResponse(
              newContext.messages,
            );
            generating = false;

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
