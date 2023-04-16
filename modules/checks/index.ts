import { Client, ColorResolvable, MessageEmbed } from 'discord.js'

import config, { Checks } from './checks.config'
import data from 'data.json'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('messageCreate', async message => {
    if (!message.channel.type.includes('GUILD') || message.author.bot) return

    let getLink = ''
    let originalLink = ''
    config.checks.every((element: Checks) => {
      const match = element.regex.exec(message.content)
      if (match != null) {
        getLink = element.getLink.replace('{code}', match[1])
        originalLink = match[0]
        return false
      } else {
        return true
      }
    })
    if (!getLink) return
    let response = ''
    try {
      // console.log(`Getting pastebin ${getLink}`);
      response = (await (await fetch(getLink)).json())
    } catch (e: any) {
      if (e.response) {
        if (e.response.status === 404) {
          await message.channel.send({
            embeds: [new MessageEmbed()
              .setTitle('Invalid Paste!')
              .setColor('#FF0000')
              .setDescription('The paste link you sent in is invalid or expired, please check the link or paste a new one.')
              .setFooter({ text: `${originalLink} | Sent by ${message.author.username}` })]
          })
        }
      }
      return
    }
    if (!response) return
    for (const test of config.tests) {
      let cause: RegExpExecArray | null = null
      for (const check of test.checks) {
        const match = check.exec(response)
        if (match) {
          cause = match
        }
      }
      if (cause) {
        const embed = new MessageEmbed()
        embed.setTitle(test.title)
        // if (test.description) embed.setDescription(test.description)
        if (test.link) {
          embed.addFields([
            { name: 'Read More', value: test.link },
            { name: 'Caused By', value: `\`\`\`${cause}\`\`\`` }
          ])
        }
        embed.setFooter({ text: `${originalLink} | Sent by ${message.author.username}` })
        embed.setColor(data.accent_color as ColorResolvable)
        await message.channel.send({ embeds: [embed] })
      }
    }
  })
}
