import { Client, ColorResolvable, MessageEmbed } from 'discord.js'
import data from '../data.json'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('messageCreate', async msg => {
    // Ignore ourself
    if (msg.author.bot) return
    // Ignore DMs
    if (msg.channel.type !== 'GUILD_TEXT') return

    // If the stripped message contains skinrestorer
    if (msg.content.toLowerCase().replace(/\W/gm, '').includes('skinrestorer')) {
      await msg.channel.send({
        embeds: [new MessageEmbed()
          .setTitle('It looks like you\'re trying to spell SkinsRestorer!')
          .setDescription('A useful tip to remember how to spell it is: **SKINS** is not **SKIN**')
          .setColor(data.accent_color as ColorResolvable)
          .setThumbnail('https://www.spigotmc.org/data/resource_icons/2/2124.jpg')]
      })
    }
  })
}
