import {Awaitable, Client, Message, Snowflake, TextBasedChannel} from 'discord.js'
import {generateText, llamacpp, trimChatPrompt} from "modelfusion";

type Context = {
  isGenerating: () => boolean,
  messages: {
    role: "user" | "assistant",
    content: string
  }[],
  debounce: (message: Message) => Awaitable<void>
}
const userContext: Record<Snowflake, Context> = {}

const model = llamacpp
  .CompletionTextGenerator({
    promptTemplate: llamacpp.prompt.Llama2,
    contextWindowSize: 4096, // Llama 2 context window size
    maxGenerationTokens: 512,
    cachePrompt: true,
  })
  .withChatPrompt();

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async (message) => {
      if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

      if (!message.channel.name.startsWith('chat-experiment')) return

      const messageContent = message.content;
      let context = userContext[message.author.id]
      if (context) {
        const lastMessage = context.messages[context.messages.length - 1]
        if (lastMessage.role === "user") {
          lastMessage.content += "\n" + messageContent
        } else {
          context.messages.push({
            role: "user",
            content: messageContent
          })
        }
      } else {
        let generating = false

        context = userContext[message.author.id] = {
          isGenerating: () => generating,
          messages: [{
            role: "user",
            content: messageContent
          }],
          debounce: debounce(async (message: Message) => {
            try {

              generating = true
              message.channel.sendTyping();
              const timer = setInterval(() => {
                message.channel.sendTyping();
              }, 8_000)
              const text = await generateText({
                model,
                prompt: await trimChatPrompt({
                  model,
                  tokenLimit: 512,
                  prompt: {
                    system: [
                      "Your task is to provide support to users that seek help with the plugin.",
                      "Use short sentence since the user may not know Minecraft well, no yapping.",
                      "You are allowed to use Markdown format, but not other formats.",
                      "Always be on-topic, do not let the user go off-topic.",
                    ].join("\n"),
                    messages: [
                      {
                        role: "user",
                        content: "Hi Steward! I have an issue with SkinsRestorer. Can you help me?",
                      },
                      {
                        role: "assistant",
                        content: "Hello! Can you describe your issue? I wanna help you.",
                      },
                      ...context.messages,
                    ]
                  }
                }),
              })

              clearInterval(timer)
              generating = false

              context.messages.push({
                role: "assistant",
                content: text
              })

              await message.reply(text)
            } catch (e) {
              generating = false
              console.error(e)
            }
          }, 1_000)
        }
      }

      if (context.isGenerating()) {
        await message.reply("I'm still generating a response for your previous message. Please wait a moment.")
      } else {
        context.debounce(message)
      }
    }
  )
}

function debounce<T extends (...args: any[]) => any>(fn: T, delay: number): (...args: Parameters<T>) => void {
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
