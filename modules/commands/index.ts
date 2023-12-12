import {Client, ColorResolvable, EmbedBuilder, REST, Routes, SlashCommandBuilder} from 'discord.js'

import config from '../../config.json'
import data from 'data.json'
import {ConfigCommand, configCommands} from "./commands.config";

const commands: ConfigCommand[] = configCommands.sort((a, b) => {
  if (a.name < b.name) return -1
  if (a.name > b.name) return 1
  return 0
})

let metaData: { name?: string } = {}

const fetchData = async (): Promise<void> => {
  try {
    metaData = {...await (await fetch('https://api.spiget.org/v2/resources/2124/versions/latest')).json()}
  } catch (e) {
    console.error(e)
  }
}

await fetchData()
setInterval(fetchData, 60000)

const slashApiCommands: any[] = [
  new SlashCommandBuilder()
    .setName("help")
    .setDescription("Show Steward help")
    .addUserOption(option => option.setName('user').setDescription('Mention a specific user with the command'))
    .toJSON(),
  new SlashCommandBuilder()
    .setName("latest")
    .setDescription("Show latest version on SpigotMC")
    .addUserOption(option => option.setName('user').setDescription('Mention a specific user with the command'))
    .toJSON()
]

for (const command of commands) {
  const slashCommand = new SlashCommandBuilder()
    .setName(command.name)
    .setDescription(command.cmdDescription)
    .addUserOption(option => option.setName('user').setDescription('Mention a specific user with the command'))

  slashApiCommands.push(slashCommand.toJSON())
}

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  const rest = new REST().setToken(config.token)

  console.log(`Started refreshing ${slashApiCommands.length} application (/) commands.`)

  // The put method is used to fully refresh all commands in the guild with the current set
  const responseData = await rest.put(
    Routes.applicationCommands(config.clientId),
    {body: slashApiCommands}
  ) as any

  console.log(`Successfully reloaded ${responseData.length} application (/) commands.`)

  client.on('interactionCreate', async interaction => {
    if (!interaction.isChatInputCommand()) return

    // Grab the command
    const trigger = interaction.commandName

    // Ignore if trigger is blank
    if (trigger === '') return

    const targetUser = interaction.options.getUser('user')

    // Initiate the embed
    const embed = new EmbedBuilder()

    if (trigger === 'help') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Steward help')
        .setDescription("Hi! :wave: I am Steward. Here to help out at SkinsRestorer. The code for commands can be [found on GitHub](https://github.com/SkinsRestorer/steward/tree/main/modules/commands)")

      await interaction.reply({embeds: [embed], ephemeral: true})
      return
    }

    if (trigger === 'latest') {
      embed
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Latest version')
        .setDescription(`\`${metaData.name ?? 'Unknown'}\``)

      await interaction.reply({embeds: [embed]})
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
        .setFooter({
          text: 'SkinsRestorer documentation',
          iconURL: 'https://skinsrestorer.net/logo.png'
        })

      if (item.url != null) {
        embed.addFields([{name: 'Read more', value: item.url}])
      }
    } else {
      embed.setTitle(`${item.title}`)

      if (item.url != null) {
        embed.addFields([{name: 'Link', value: item.url}])
      }
    }

    if (item.fields != null) {
      item.fields.forEach(field => {
        embed.addFields([{name: field.key, value: field.value, inline: false}])
      })
    }

    let message
    if (targetUser != null) {
      message = `<@${targetUser.id}>`
    }

    await interaction.reply({content: message, embeds: [embed]})
  })
}
