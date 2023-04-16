import { Client, ColorResolvable, EmbedBuilder } from 'discord.js'
import data from '../data.json'

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    // Ignore ourself
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    // If the stripped message contains skinrestorer
    if (message.content.toLowerCase().replace(/\W/gm, '').includes('skinrestorer')) {
      await message.reply({
        embeds: [new EmbedBuilder()
          .setTitle('It looks like you\'re trying to spell SkinsRestorer!')
          .setDescription('A useful tip to remember how to spell it is: **SKINS** is not **SKIN**')
          .setColor(data.accent_color as ColorResolvable)
          .setThumbnail('https://www.spigotmc.org/data/resource_icons/2/2124.jpg')]
      })
    }
  })
}
