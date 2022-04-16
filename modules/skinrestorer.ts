import { Client, MessageEmbed } from 'discord.js'

export default (client: Client) => {
  client.on('message', async msg => {
    // Ignore ourself
    if (msg.author.bot) return
    // Ignore DMs
    if (msg.channel.type !== 'GUILD_TEXT') return

    // If the stripped message contains skinrestorer
    if (msg.content.toLowerCase().replace(/\W/gm, '').includes('skinrestorer')) {
      await msg.channel.send({
        embeds: [new MessageEmbed()
          .setTitle('It looks like you\'re trying to spell SkinsRestorer!')
          .setDescription('A useful tip to remember how to spell it is: SKINS is not SKIN')
          .setThumbnail('https://www.spigotmc.org/data/resource_icons/2/2124.jpg')]
      })
    }
  })
}
