import type { ModelMessage } from "ai";
import type { Client, Message } from "discord.js";
import type { BotConfig } from "@/bot-config";
import { generateSupportResponse, isPromptInjectionAttempt } from "@/lib/ai";

type Context = {
  isGenerating: () => boolean;
  messages: ModelMessage[];
  queueFollowUp: (message: Message) => void;
  debounce: (message: Message) => void;
};

// noinspection JSUnusedGlobalSymbols
export default async (client: Client, bot: BotConfig): Promise<void> => {
  const config = bot.chatbot;
  if (config == null) {
    return;
  }

  const userContext: Record<string, Context> = {};

  client.on("messageCreate", async (message) => {
    const channel = message.channel;
    if (!channel.isTextBased() || channel.isDMBased() || message.author.bot)
      return;

    if (
      !config.channelNamePrefixes.some((prefix) =>
        channel.name.startsWith(prefix),
      )
    )
      return;

    const sendReply = async (
      message: Message,
      content: string,
    ): Promise<boolean> => {
      const replyContent = prefixWithMention(message, content);

      if (!message.system) {
        try {
          await message.reply(replyContent);
          return true;
        } catch (error) {
          console.error(error);
        }
      }

      try {
        await channel.send(replyContent);
        return true;
      } catch (error) {
        console.error(error);
        return false;
      }
    };

    const messageContent = message.content.trim();
    if (messageContent === "") return;

    if (isPromptInjectionAttempt(messageContent, config.ai)) {
      await sendReply(message, config.promptInjectionErrorMessage);
      return;
    }

    const contextKey = `${bot.id}:${channel.id}:${message.author.id}`;
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
            const { responseMessages, text } = await generateSupportResponse(
              requestMessages,
              config.ai,
              {
                maxLength: config.maxResponseLength,
              },
            );

            if (await sendReply(message, text)) {
              newContext.messages.push(...responseMessages);
            }
          } catch (e) {
            console.error(e);
            if (await sendReply(message, config.generationErrorMessage)) {
              newContext.messages.push({
                role: "assistant",
                content: config.generationErrorMessage,
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

function prefixWithMention(message: Message, content: string): string {
  const mention = `<@${message.author.id}>`;
  if (content.startsWith(mention)) {
    return content;
  }

  return `${mention} ${content}`;
}
