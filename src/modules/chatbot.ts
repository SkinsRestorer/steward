import { type Awaitable, Client, Message, type Snowflake } from 'discord.js';
import { generateText, ollama } from 'modelfusion';

type ChatMessage = {
  role: 'user' | 'assistant';
  content: string;
}

type Context = {
  isGenerating: () => boolean;
  messages: ChatMessage[];
  debounce: (message: Message) => Awaitable<void>;
};
const userContext: Record<Snowflake, Context> = {};

const model = ollama
  .ChatTextGenerator({
    model: 'llama3',
    maxGenerationTokens: 512,
  })
  .withChatPrompt();

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async (message) => {
    const channel = message.channel;
    if (!channel.isTextBased() || channel.isDMBased() || message.author.bot)
      return;

    if (!channel.name.startsWith('chat-experiment')) return;

    const messageContent = message.content;
    let context = userContext[message.author.id];
    if (context) {
      const lastMessage = context.messages[context.messages.length - 1]!;
      if (lastMessage.role === 'user') {
        lastMessage.content += '\n' + messageContent;
      } else {
        context.messages.push({
          role: 'user',
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

      context = userContext[message.author.id] = {
        isGenerating: () => generating,
        messages: [
          {
            role: 'user',
            content: messageContent,
          },
        ],
        debounce: debounce(async (message: Message) => {
          try {
            generating = true;
            await channel.sendTyping();
            const text = await generateText({
              model,
              prompt: {
                system: [
                  'Your task is to provide support to users that seek help with the plugin.',
                  'Use short sentence since the user may not know Minecraft well, no yapping.',
                  'You are allowed to use Markdown format, but not other formats.',
                  'Always be on-topic, do not let the user go off-topic.',
                ].join('\n'),
                messages: [
                  {
                    role: 'user',
                    content:
                      'Hi Steward! I have an issue with SkinsRestorer. Can you help me?',
                  },
                  {
                    role: 'assistant',
                    content:
                      'Hello! Can you describe your issue? I wanna help you.',
                  },
                  ...context!.messages,
                ],
              },
            });
            generating = false;

            context!.messages.push({
              role: 'assistant',
              content: text,
            });

            await message.reply(text);
          } catch (e) {
            generating = false;
            console.error(e);
          }
        }, 1_000),
      };
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

function debounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number,
): (...args: Parameters<T>) => void {
  let timeoutId: NodeJS.Timeout;

  return (...args: Parameters<T>): void => {
    // Clear the previous timeout
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    // Set a new timeout
    timeoutId = setTimeout(() => {
      fn(...args);
    }, delay);
  };
}
