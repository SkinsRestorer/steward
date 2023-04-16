import discord, { Client, ColorResolvable } from 'discord.js'

// Import commands and sort by alphabetical order (for !help command)
import list from './list.json'
import data from 'data.json'

const commands = list.sort((a, b) => {
  if (a.name < b.name) return -1
  if (a.name > b.name) return 1
  return 0
})

// For !help command
const splitCommands = (start: number, end?: number): string => {
  return commands.slice(start, end).reduce((prev, command) => {
    return (prev !== '') ? prev + `\n\`!${command.name}\`` : `\`!${command.name}\``
  }, '')
}

const leftList = splitCommands(0, Math.ceil(commands.length / 2))
const rightList = splitCommands(Math.ceil(commands.length / 2))

let metaData: { name?: string } = {}

const fetchData = async (): Promise<void> => {
  try {
    metaData = { ...await (await fetch('https://api.spiget.org/v2/resources/2124/versions/latest')).json() }
  } catch (e) {
    console.error(e)
  }
}

await fetchData()
setInterval(fetchData, 60000)

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    // Ignore DMs and messages that don't start with the prefix
    if (!message.channel.type.includes('GUILD') || message.author.bot) return

    if (!message.content.startsWith('!') || message.author.bot) return

    // Grab the command
    const trigger = message.content.toLowerCase().substring(1).split(' ')[0].replace(/[^0-9a-z]/gi, '')

    // Ignore if trigger is blank
    if (trigger === '') return

    // Initiate the embed
    const embed = new discord.MessageEmbed()

    // !help command
    if (trigger === 'help') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Available commands:')
        .addFields([
          { name: '\u200E', value: leftList, inline: true },
          { name: '\u200E', value: rightList, inline: true },
          { name: '\u200E', value: '`!latest`', inline: true }
        ])

      await message.channel.send({ embeds: [embed] })
      return
    }

    if (trigger === 'latest') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Latest version')
        .setDescription('`' + (metaData.name ?? 'Unknown') + '`')

      await message.channel.send({ embeds: [embed] })
      return
    }

    // Check if command name exists
    let item = commands.find(command => {
      return command.name === trigger
    })

    // Check for an alias
    if (item == null) {
      item = commands.find(command => {
        if (command.aliases == null) return false

        return command.aliases.includes(trigger)
      })
    }

    // If no command found, throw an error
    if (item === null || item === undefined) {
      await message.channel.send(`Sorry! I do not understand the command \`!${trigger}\`\nType \`!help\` for a list of commands.`)
      return
    }

    // Begin formatting the command embed
    embed
      .setColor(data.accent_color as ColorResolvable)
      .setDescription(item.description)

    if (item.url != null) {
      embed.setURL(item.url)
    }

    if (item.wiki === true) {
      embed
        .setTitle(`🔖 ${item.title}`)
        .setFooter({ text: 'SkinsRestorer wiki', iconURL: 'https://www.spigotmc.org/data/resource_icons/2/2124.jpg' })

      if (item.url != null) {
        embed.addFields([{ name: 'Read more', value: item.url }])
      }
    } else {
      embed.setTitle(`${item.title}`)

      if (item.url != null) {
        embed.addFields([{ name: 'Link', value: item.url }])
      }
    }

    if (item.fields != null) {
      item.fields.forEach(field => {
        embed.addFields([{ name: field.key, value: field.value, inline: false }])
      })
    }

    await message.channel.send({ embeds: [embed] })
  })
}
