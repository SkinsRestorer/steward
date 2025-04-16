import { Client } from 'discord.js'

import data from 'data.json'

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    if (message.mentions.members?.size === 0) return
    if (message.reference !== null) return

    const senderIsStaff = message.member?.roles.cache.some(role => data.staff_roles.includes(role.name))
    if (senderIsStaff === true) {
      return
    }

    const senderIsBot = message.author.bot
    if (senderIsBot) {
      return
    }

    const mentionsStaff = message.mentions.members?.some(member => {
      // If the message mentions any members that satisfy the following:
      return member.roles.cache.some(role => data.staff_roles.includes(role.name))
    })

    if (mentionsStaff === true) {
      // Tell them off:
      await message.reply(`Hey ${message.member?.nickname ?? message.author.username}! Please don't tag staff members directly.`)
    }
  })
}
