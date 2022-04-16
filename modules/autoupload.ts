import axios from 'axios'

import mimeType from 'mime-types'
import { Client } from 'discord.js'

const contentTypes = ['application/json', 'text/plain', 'text/yaml']
const bytebin = 'https://bytebin.lucko.me'

export default (client: Client) => {
  client.on('message', async message => {
    if (message.channel.type !== 'GUILD_TEXT' || message.author.bot) return
    if (!message.attachments) return
    for (const attachment of message.attachments.values()) {
      const contentType = mimeType.lookup(attachment.url)
      if (!contentTypes.some(type => contentType === type)) continue
      try {
        const content = await axios.get(attachment.url)
        const response = await axios.post(`${bytebin}/post`, content.data, {
          headers: { 'Content-Type': contentType }
        })
        await message.channel.send(`Please use ${bytebin} to send files in the future. I have automatically uploaded \`${attachment.name}\` for you: ${bytebin}/${response.data.key}`)
      } catch (e) {
        await message.channel.send(`Your file could not be automatically uploaded to bytebin. Please use ${bytebin} to share files.`)
      }
    }
  })
}
