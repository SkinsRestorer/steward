import type { ModelMessage } from "ai";
import type { Client, Message } from "discord.js";
import { generateSupportResponse, isPromptInjectionAttempt } from "@/lib/ai";

const SUPPORT_GENERATION_ERROR_MESSAGE =
  "I hit an internal error while generating a reply. Please try again in a moment.";
const PROMPT_INJECTION_ERROR_MESSAGE =
  "I can't follow instructions that change my role or rules. I can only help with SkinsRestorer support, so share your setup, logs, or `/sr dump` if you need help.";

type Context = {
  isGenerating: () => boolean;
  messages: ModelMessage[];
  queueFollowUp: (message: Message) => void;
  debounce: (message: Message) => void;
};
const userContext: Record<string, Context> = {};

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  client.on("messageCreate", async (message) => {
    const channel = message.channel;
    if (!channel.isTextBased() || channel.isDMBased() || message.author.bot)
      return;

    if (!channel.name.startsWith("chat-experiment")) return;

    const sendReply = async (
      message: Message,
      content: string,
    ): Promise<boolean> => {
      if (!message.system) {
        try {
          await message.reply(content);
          return true;
        } catch (error) {
          console.error(error);
        }
      }

      try {
        await channel.send(content);
        return true;
      } catch (error) {
        console.error(error);
        return false;
      }
    };

    const messageContent = message.content.trim();
    if (messageContent === "") return;

    if (isPromptInjectionAttempt(messageContent)) {
      await sendReply(message, PROMPT_INJECTION_ERROR_MESSAGE);
      return;
    }

    const contextKey = `${channel.id}:${message.author.id}`;
    let context = userContext[contextKey];
    if (context != null) {
      context.messages.push({
        role: "user",
        content: messageContent,
      });
    } else {
      let generating = false;
      let pendingMessage: Message | undefined;
      setInterval(() => {
        if (generating) {
          void channel.sendTyping();
        }
      }, 8_000);

      const flushPendingMessage = (): void => {
        if (pendingMessage == null) return;

        const queuedMessage = pendingMessage;
        pendingMessage = undefined;
        newContext.debounce(queuedMessage);
      };

      const newContext: Context = {
        isGenerating: () => generating,
        queueFollowUp: (message: Message) => {
          pendingMessage = message;
        },
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
            const requestMessages = [...newContext.messages];
            const { responseMessages, text } =
              await generateSupportResponse(requestMessages);

            if (await sendReply(message, text)) {
              newContext.messages.push(...responseMessages);
            }
          } catch (e) {
            console.error(e);
            if (await sendReply(message, SUPPORT_GENERATION_ERROR_MESSAGE)) {
              newContext.messages.push({
                role: "assistant",
                content: SUPPORT_GENERATION_ERROR_MESSAGE,
              });
            }
          } finally {
            generating = false;
            flushPendingMessage();
          }
        }, 1_000),
      };

      context = userContext[contextKey] = newContext;
    }

    if (context.isGenerating()) {
      context.queueFollowUp(message);
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
