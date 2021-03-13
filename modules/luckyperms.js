const discord = require('discord.js')

module.exports = (client) => {
  client.on('message', async msg => {
    // Ignore ourself
    if (msg.author.bot) return
    // Ignore DMs
    if (msg.channel.type !== 'text') return

    // If the stripped message contains luckyperms
    if (msg.content.toLowerCase().replace(/\W/gm, '').indexOf('skinrestorer') !== -1) {
      await msg.channel.send(new discord.MessageEmbed()
        .setTitle('It looks like you\'re trying to spell SkinsRestorer!')
        .setDescription('A useful tip to remember how to spell it is: SKINS is not SKIN')
        .setThumbnail('https://www.spigotmc.org/data/resource_icons/2/2124.jpg')
      )
    }
  })
}
