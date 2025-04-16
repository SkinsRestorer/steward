import { Client } from 'discord.js'
import jsyaml from 'js-yaml'
import { DOMParser } from 'xmldom';

const contentTypes = ['application/json', 'application/yaml', 'text/xml', 'text/plain']
const website = 'https://pastes.dev'
const api = 'https://api.pastes.dev'

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    if (message.attachments.size === 0) return

    for (const attachment of message.attachments.values()) {
      try {
        if (attachment.contentType === null) {
          continue
        }

        let found = false
        for (const type of contentTypes) {
          if (attachment.contentType.includes(type)) {
            found = true
            break
          }
        }

        if (!found) {
          continue
        }

        const content = (await (await fetch(attachment.url)).text())
        const contentType = detectTextFormat(content) ?? attachment.contentType

        const response = (await (await fetch(`${api}/post`, {
          method: 'POST',
          body: content,
          headers: {
            'Content-Type': contentType,
            'User-Agent': 'SkinsRestorerSteward'
          }
        })).json()) as { key: string }
        await message.reply(`Please use <${website}> to send files in the future. I have automatically uploaded \`${attachment.name}\` for you: ${website}/${response.key}`)
      } catch (e) {
        console.error(e)
        await message.reply(`Your file could not be automatically uploaded. Please use ${website} to share files.`)
      }
    }
  })
}

function detectTextFormat (text: string): string | null {
  // Trim leading/trailing whitespace
  text = text.trim()

  // Check if it's JSON
  if (text.startsWith('{') && text.endsWith('}')) {
    try {
      JSON.parse(text)
      return 'text/json' // Required name for pastes.dev
    } catch (error) {
      // Not valid JSON
    }
  }

  // Check if it's YAML
  try {
    jsyaml.load(text)
    return 'text/yaml' // Required name for pastes.dev
  } catch (error) {
    // Not valid YAML
  }

  // Check if it's XML
  if (text.startsWith('<') && text.endsWith('>')) {
    try {
      // Using DOMParser to parse XML
      new DOMParser().parseFromString(text, 'text/xml')
      return 'text/xml'
    } catch (error) {
      // Not valid XML
    }
  }

  // Unknown format
  return null
}
