import { Client, ColorResolvable, EmbedBuilder, REST, Routes, SlashCommandBuilder } from 'discord.js'

// Import commands and sort by alphabetical order (for /help command)
import list from './list.json'
import config from '../../config.json'
import data from 'data.json'

interface ConfigCommand {
  name: string
  url?: string
  title: string
  description: string
  docs?: boolean
  fields?: CommandField[]
}

interface CommandField {
  key: string
  value: string
}

const commands: ConfigCommand[] = list.sort((a, b) => {
  if (a.name < b.name) return -1
  if (a.name > b.name) return 1
  return 0
})

// For !help command
const splitCommands = (start: number, end?: number): string => {
  return commands.slice(start, end).reduce((prev, command) => {
    return (prev !== '') ? prev + `\n\`/${command.name}\`` : `\`/${command.name}\``
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

const slashApiCommands: any[] = []
for (const command of commands) {
  const slashCommand = new SlashCommandBuilder()
    .setName(command.name)
    .setDescription("Support help command")

  slashApiCommands.push(slashCommand.toJSON())
}

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  const rest = new REST().setToken(config.token)

  console.log(`Started refreshing ${commands.length} application (/) commands.`)

  // The put method is used to fully refresh all commands in the guild with the current set
  const responseData = await rest.put(
    Routes.applicationCommands(config.clientId),
    { body: slashApiCommands }
  ) as any

  console.log(`Successfully reloaded ${responseData.length} application (/) commands.`)

  client.on('interactionCreate', async interaction => {
    if (!interaction.isChatInputCommand()) return

    // Grab the command
    const trigger = interaction.commandName

    // Ignore if trigger is blank
    if (trigger === '') return

    // Initiate the embed
    const embed = new EmbedBuilder()

    // /help command
    if (trigger === 'help') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Available commands:')
        .addFields([
          { name: '\u200E', value: leftList, inline: true },
          { name: '\u200E', value: rightList, inline: true },
          { name: '\u200E', value: '`/latest`', inline: true }
        ])

      await interaction.reply({ embeds: [embed], ephemeral: true })
      return
    }

    if (trigger === 'latest') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Latest version')
        .setDescription('`' + (metaData.name ?? 'Unknown') + '`')

      await interaction.reply({ embeds: [embed] })
      return
    }

    // Check if command name exists
    const item = commands.find(command => {
      return command.name === trigger
    })

    // If no command found, throw an error
    if (item === null || item === undefined) {
      await interaction.reply(`Sorry! I do not understand the command \`!${trigger}\`\nType \`!help\` for a list of commands.`)
      return
    }

    // Begin formatting the command embed
    embed
      .setColor(data.accent_color as ColorResolvable)
      .setDescription(item.description)

    if (item.url != null) {
      embed.setURL(item.url)
    }

    if (item.docs === true) {
      embed
        .setTitle(`🔖 ${item.title}`)
        .setFooter({ text: 'SkinsRestorer documentation', iconURL: 'https://www.spigotmc.org/data/resource_icons/2/2124.jpg' })

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

    await interaction.reply({ embeds: [embed] })
  })
}
