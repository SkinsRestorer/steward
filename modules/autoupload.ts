import {Client} from 'discord.js'

const contentTypes = ['application/json', 'text/plain', 'text/yaml']
const website = 'https://pastes.dev'
const api = 'https://api.pastes.dev'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    if (message.attachments.size === 0) return

    for (const attachment of message.attachments.values()) {
      try {
        console.log(attachment.contentType)
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
        const response = (await (await fetch(`${api}/post`, {
          method: 'POST',
          body: content,
          headers: {
            'Content-Type': attachment.contentType
          }
        })).json())
        await message.channel.send(`Please use <${website}> to send files in the future. I have automatically uploaded \`${attachment.name}\` for you: ${website}/${response.key}`)
      } catch (e) {
        console.error(e)
        await message.channel.send(`Your file could not be automatically uploaded. Please use ${website} to share files.`)
      }
    }
  })
}
