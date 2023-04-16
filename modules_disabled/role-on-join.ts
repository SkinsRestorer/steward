import data from '../data.json'
import { Client } from 'discord.js'

export default (client: Client) => {
  client.on('guildMemberAdd', async (member) => {
    await member.roles.add(data.member_role, 'Autorole')
  })
}
