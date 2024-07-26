import { Client, ColorResolvable, EmbedBuilder } from 'discord.js'
import data from '../data.json'

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    // Ignore ourself
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return
    const strippedMessage = message.content.toLowerCase().replace(/\W/gm, '')

    // If the stripped message contains skinrestorer
    if (strippedMessage.includes('skinrestorer')) {
      await message.reply({
        embeds: [new EmbedBuilder()
          .setTitle('It looks like you\'re trying to spell SkinsRestorer!')
          .setDescription('A useful tip to remember how to spell it is that we restore __many__ **SKINS**, not just one **SKIN**!')
          .setColor(data.accent_color as ColorResolvable)
          .setThumbnail('https://skinsrestorer.net/logo.png')]
      })
    }

    // If the stripped message starts with "/sr"
    if (strippedMessage.startsWith('/sr')) {
      await message.reply({
        embeds: [new EmbedBuilder()
          .setTitle('Not in Discord you fool! Run it in the server 😄')
          .setDescription('This is a server command, you run it in the server console or in the in-game chat, not in Discord!')
          .setColor(data.accent_color as ColorResolvable)]
      })
    }
  })
}
