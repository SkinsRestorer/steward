import { Client } from 'discord.js'

import data from '../data.json'

export default (client: Client) => {
  client.on('message', async msg => {
    // Ignore DMs, messages that don't mention anyone, and messages that are a reply.
    if (msg.channel.type !== 'GUILD_TEXT') return
    if (msg.mentions.members?.size === 0) return
    if (msg.reference !== null) return

    const senderIsStaff = msg.member?.roles.cache.some(role => data.staff_roles.includes(role.name))
    if (senderIsStaff) {
      return
    }

    const senderIsBot = msg.author.bot
    if (senderIsBot) {
      return
    }

    const mentionsStaff = msg.mentions.members?.some(member => {
      // If the message mentions any members that satisfy the following:
      return member.roles.cache.some(role => data.staff_roles.includes(role.name))
    })

    if (mentionsStaff) {
      // Tell them off:
      await msg.channel.send(`Hey ${msg.member?.nickname ?? msg.author.username}! Please don't tag staff members directly.`)
    }
  })
}
