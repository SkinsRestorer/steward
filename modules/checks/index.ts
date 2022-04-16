import { Client, MessageEmbed } from 'discord.js'

import axios from 'axios'
import config, { Checks } from './checks.config'

export default (client: Client) => {
  client.on('message', async message => {
    if (message.channel.type !== 'GUILD_TEXT') return
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
      response = (await axios.get(getLink)).data
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
        if (test.link) embed.addField('Read More', test.link)
        embed.addField('Caused By', `\`\`\`${cause}\`\`\``)
        embed.setFooter(`${originalLink} | Sent by ${message.author.username}`)
        embed.setColor('#96dd35')
        await message.channel.send({ embeds: [embed] })
      }
    }
  })
}
