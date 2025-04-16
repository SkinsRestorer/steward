import {
  ActionRowBuilder,
  type ActionRowData,
  ApplicationCommandType,
  Client,
  type ColorResolvable,
  ComponentType,
  ContextMenuCommandBuilder,
  EmbedBuilder,
  InteractionContextType,
  type MessageActionRowComponentData,
  PermissionFlagsBits,
  REST,
  type RESTPostAPIApplicationCommandsJSONBody,
  Routes,
  SlashCommandBuilder,
  StringSelectMenuBuilder,
  StringSelectMenuOptionBuilder,
} from 'discord.js'

import data from 'data.json'
import {type ConfigCommand, configCommands} from './commands-config'
import {getMetadata} from './metadata'

const commands: ConfigCommand[] = configCommands.sort((a, b) => {
  if (a.name < b.name) return -1
  if (a.name > b.name) return 1
  return 0
})

const slashApiCommands: RESTPostAPIApplicationCommandsJSONBody[] = [
  new SlashCommandBuilder()
    .setName('help')
    .setDescription('Show Steward help')
    .addUserOption(option => option.setName('user').setDescription('Mention a specific user with the command'))
    .toJSON(),
  new SlashCommandBuilder()
    .setName('latest')
    .setDescription('Show latest version on GitHub')
    .addUserOption(option => option
      .setName('user')
      .setDescription('Mention a specific user with the command')
    )
    .toJSON(),
  new SlashCommandBuilder()
    .setName('resolved')
    .setDescription('Moderator command to mark a forum post as resolved')
    .setDefaultMemberPermissions(PermissionFlagsBits.ManageThreads)
    .setContexts([InteractionContextType.Guild])
    .toJSON()
]

for (const command of commands) {
  const slashCommand = new SlashCommandBuilder()
    .setName(command.name)
    .setDescription(command.cmdDescription)
    .addUserOption(option => option
      .setName('user')
      .setDescription('Mention a specific user with the command')
    )

  slashApiCommands.push(slashCommand.toJSON())
}

const sendHelpContext =
  new ContextMenuCommandBuilder()
    .setName("Send Help")
    .setType(ApplicationCommandType.Message)
    .toJSON();
slashApiCommands.push(sendHelpContext)

interface CommandData {
  id: string
  description: string
  private: boolean
}

export const commandIdRegistry: Record<string, CommandData> = {}

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  const rest = new REST().setToken(process.env.DISCORD_TOKEN!)

  console.log(`Started refreshing ${slashApiCommands.length} application (/) commands.`)

  // The put method is used to fully refresh all commands in the guild with the current set
  const responseData = await rest.put(
    Routes.applicationCommands(data.clientId),
    {body: slashApiCommands}
  ) as any

  for (const response of responseData) {
    commandIdRegistry[response.name] = {
      id: response.id,
      description: response.description,
      private: !!response.defaultPermission
    }
  }

  console.log(`Successfully reloaded ${responseData.length} application (/) commands.`)

  client.on('interactionCreate', async interaction => {
    if (!interaction.isChatInputCommand() && !interaction.isMessageContextMenuCommand()) return

    // Get the trigger
    let trigger: string
    if (interaction.isChatInputCommand()) {
      trigger = interaction.commandName
    } else if (interaction.isMessageContextMenuCommand()) {
      if (interaction.commandName === sendHelpContext.name) {
        const customId = `help-selection-${Date.now()}`
        const select = new StringSelectMenuBuilder()
          .setCustomId(customId)
          .setPlaceholder('Choose a help type!')
          .addOptions(commands.map(command => new StringSelectMenuOptionBuilder()
            .setLabel(command.name)
            .setValue(command.name)
            .setDescription(command.cmdDescription)
          ))

        const row = new ActionRowBuilder()
          .addComponents(select)
          .toJSON() as ActionRowData<MessageActionRowComponentData>;

        const final = await interaction.reply({
          ephemeral: true,
          content: 'Please select a help type!',
          components: [row]
        })

        try {
          const component = await final.awaitMessageComponent({
            time: 1000 * 60,
            componentType: ComponentType.StringSelect,
            filter: (i) => i.user.id === interaction.user.id && i.customId === customId
          })

          trigger = component.values[0]
          await interaction.deleteReply()
        } catch (e) {
          await interaction.editReply({content: "Timed out", components: []})
          return
        }
      } else {
        await interaction.reply('Unknown command')
        return
      }
    } else {
      return
    }

    // Ignore if trigger is blank
    if (trigger === '') return

    if (trigger === 'resolved') {
      const channel = interaction.channel
      if (channel == null || !channel.isThread()) {
        await interaction.reply('This command can only be used in threads.')
        return
      }

      if (channel.appliedTags.includes(data.resolvedTag)) {
        await interaction.reply('This thread is already marked as resolved.')
        return
      }

      // Send a message before locking the thread, so we can still reply to it
      await interaction.reply('This thread has been marked as resolved, locked and archived. Thank you for using SkinsRestorer! For any future issues, please create a new post.')
      await channel.setAppliedTags([...channel.appliedTags, data.resolvedTag])
      await channel.setLocked(true)
      await channel.setArchived(true, 'resolved')

      return
    }

    if (trigger === 'help') {
      const embed = new EmbedBuilder()
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Steward help')
        .setDescription('Hi! :wave: I am Steward. Here to help out at SkinsRestorer. The code for steward can be [found on GitHub](https://github.com/SkinsRestorer/steward)')
        .addFields(Object.entries(commandIdRegistry)
          .filter(([, data]) => !data.private)
          .map(([name, data]) => {
            return {
              name: `</${name}:${data.id}>`,
              value: data.description,
              inline: true
            }
          }))

      await interaction.reply({embeds: [embed], ephemeral: true})
      return
    }

    if (trigger === 'latest') {
      const embed = new EmbedBuilder()
        .setColor(data.accent_color as ColorResolvable)
        .setTitle('Latest version')
        .setDescription(`\`${getMetadata().tag_name ?? 'Unknown'}\``)

      await interaction.reply({embeds: [embed]})
      return
    }

    // Check if command name exists
    const item = commands.find(command => {
      return command.name === trigger
    })

    // If no command found, throw an error
    if (item === null || item === undefined) {
      await interaction.reply("Something went wrong! I couldn't find that command.")
      return
    }

    const embed = new EmbedBuilder()

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

    if (interaction.isChatInputCommand()) {
      const targetUser = interaction.isChatInputCommand() ? interaction.options.getUser('user') : null
      let message
      if (targetUser != null) {
        message = `<@${targetUser.id}>`
      }

      await interaction.reply({content: message, embeds: [embed]})
    } else if (interaction.isContextMenuCommand()) {
      const message = `Requested by <@${interaction.user.id}>`
      await interaction.targetMessage.reply({content: message, embeds: [embed]})
    }
  })
}
