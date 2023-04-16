import mimeType from 'mime-types'
import { Client } from 'discord.js'

const contentTypes = ['application/json', 'text/plain', 'text/yaml']
const bytebin = 'https://bytebin.lucko.me'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    if (!message.attachments) return
    for (const attachment of message.attachments.values()) {
      const contentType = mimeType.lookup(attachment.url)
      if (!contentTypes.some(type => contentType === type)) continue
      try {
        const content = (await (await fetch(attachment.url)).json())
        const response = (await (await fetch(`${bytebin}/post`, {
          method: 'POST',
          body: content,
          headers: {
            'Content-Type': String(contentType)
          }
        })).json())
        await message.channel.send(`Please use ${bytebin} to send files in the future. I have automatically uploaded \`${attachment.name}\` for you: ${bytebin}/${response.key}`)
      } catch (e) {
        await message.channel.send(`Your file could not be automatically uploaded to bytebin. Please use ${bytebin} to share files.`)
      }
    }
  })
}
