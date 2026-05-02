import {
  ActionRowBuilder,
  type ActionRowData,
  ApplicationCommandType,
  type Client,
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
} from "discord.js";
import type { RESTPutAPIApplicationCommandsResult } from "discord-api-types/v10";
import type { BotConfig, CommandsConfig, ConfigCommand } from "@/bot-config";
import { generateSupportResponse } from "@/lib/ai";
import { getBotToken } from "@/lib/bot-token";
import { getLatestReleaseProvider } from "@/lib/github-release";

interface CommandData {
  id: string;
  description: string;
  private: boolean;
}

const sortCommands = (commands: ConfigCommand[]): ConfigCommand[] =>
  [...commands].sort((a, b) => {
    if (a.name < b.name) return -1;
    if (a.name > b.name) return 1;
    return 0;
  });

const buildUserTargetCommand = (
  name: string,
  description: string,
): RESTPostAPIApplicationCommandsJSONBody =>
  new SlashCommandBuilder()
    .setName(name)
    .setDescription(description)
    .addUserOption((option) =>
      option
        .setName("user")
        .setDescription("Mention a specific user with the command"),
    )
    .toJSON();

const buildSlashApiCommands = (
  config: CommandsConfig,
  commands: ConfigCommand[],
): RESTPostAPIApplicationCommandsJSONBody[] => {
  const slashApiCommands: RESTPostAPIApplicationCommandsJSONBody[] = [];

  if (config.help != null) {
    slashApiCommands.push(
      buildUserTargetCommand("help", config.help.description),
    );
  }

  if (config.latest != null) {
    slashApiCommands.push(
      buildUserTargetCommand("latest", config.latest.description),
    );
  }

  if (config.resolved != null) {
    slashApiCommands.push(
      new SlashCommandBuilder()
        .setName("resolved")
        .setDescription(config.resolved.description)
        .setDefaultMemberPermissions(PermissionFlagsBits.ManageThreads)
        .setContexts([InteractionContextType.Guild])
        .toJSON(),
    );
  }

  for (const command of commands) {
    slashApiCommands.push(
      buildUserTargetCommand(command.name, command.cmdDescription),
    );
  }

  if (config.sendHelpContext != null) {
    slashApiCommands.push(
      new ContextMenuCommandBuilder()
        .setName(config.sendHelpContext.name)
        .setType(ApplicationCommandType.Message)
        .toJSON(),
    );
  }

  if (config.sendSupportContext != null) {
    slashApiCommands.push(
      new ContextMenuCommandBuilder()
        .setName(config.sendSupportContext.name)
        .setType(ApplicationCommandType.Message)
        .toJSON(),
    );
  }

  if (config.replyWithAiContext != null) {
    slashApiCommands.push(
      new ContextMenuCommandBuilder()
        .setName(config.replyWithAiContext.name)
        .setType(ApplicationCommandType.Message)
        .toJSON(),
    );
  }

  return slashApiCommands;
};

// noinspection JSUnusedGlobalSymbols
export default async (client: Client, bot: BotConfig): Promise<void> => {
  const config = bot.commands;
  if (config == null) {
    return;
  }

  const token = getBotToken(bot);
  const commands = sortCommands(config.commandResponses);
  const slashApiCommands = buildSlashApiCommands(config, commands);
  const commandIdRegistry: Record<string, CommandData> = {};
  const latestReleaseProvider =
    config.latest == null
      ? () => null
      : getLatestReleaseProvider(config.latest.releaseUrl);

  const rest = new REST().setToken(token);

  console.log(
    `[${bot.id}] Started refreshing ${slashApiCommands.length} application (/) commands.`,
  );

  const responseData = (await rest.put(
    Routes.applicationCommands(bot.clientId),
    { body: slashApiCommands },
  )) as RESTPutAPIApplicationCommandsResult;

  for (const response of responseData) {
    commandIdRegistry[response.name] = {
      id: response.id,
      description: response.description,
      private: Boolean(response.default_permission),
    };
  }

  console.log(
    `[${bot.id}] Successfully reloaded ${responseData.length} application (/) commands.`,
  );

  client.on("interactionCreate", async (interaction) => {
    let isSendHelpFlow = false;
    if (
      !interaction.isChatInputCommand() &&
      !interaction.isMessageContextMenuCommand()
    )
      return;

    let trigger: string;
    if (interaction.isChatInputCommand()) {
      trigger = interaction.commandName;
    } else if (interaction.isMessageContextMenuCommand()) {
      if (
        config.sendHelpContext != null &&
        interaction.commandName === config.sendHelpContext.name
      ) {
        isSendHelpFlow = true;
        const customId = `help-selection-${bot.id}-${Date.now()}`;
        const select = new StringSelectMenuBuilder()
          .setCustomId(customId)
          .setPlaceholder("Choose a help type!")
          .addOptions(
            commands.map((command) =>
              new StringSelectMenuOptionBuilder()
                .setLabel(command.name)
                .setValue(command.name)
                .setDescription(command.cmdDescription),
            ),
          );

        const row = new ActionRowBuilder()
          .addComponents(select)
          .toJSON() as ActionRowData<MessageActionRowComponentData>;

        const final = await interaction.reply({
          ephemeral: true,
          content: "Please select a help type!",
          components: [row],
          fetchReply: true,
        });

        try {
          const component = await final.awaitMessageComponent({
            time: 1000 * 60,
            componentType: ComponentType.StringSelect,
            filter: (i) =>
              i.user.id === interaction.user.id && i.customId === customId,
          });

          if (!component.values[0]) {
            await interaction.editReply({
              content: "No value selected",
              components: [],
            });
            return;
          }

          trigger = component.values[0];
          await component.update({
            content: `Sending </${trigger}:${commandIdRegistry[trigger]?.id ?? "unknown"}>...`,
            components: [],
          });
        } catch (_e) {
          await interaction.editReply({ content: "Timed out", components: [] });
          return;
        }
      } else if (
        config.sendSupportContext != null &&
        interaction.commandName === config.sendSupportContext.name
      ) {
        const embed = new EmbedBuilder()
          .setColor(bot.accentColor)
          .setTitle(config.sendSupportContext.embedTitle)
          .setDescription(config.sendSupportContext.embedDescription);

        if (config.sendSupportContext.embedUrl != null) {
          embed.setURL(config.sendSupportContext.embedUrl);
        }

        const message = `<@${interaction.targetMessage.author.id}> Requested by <@${interaction.user.id}>`;

        await interaction.targetMessage.reply({
          content: message,
          embeds: [embed],
        });

        return;
      } else if (
        config.replyWithAiContext != null &&
        interaction.commandName === config.replyWithAiContext.name
      ) {
        if (interaction.targetMessage.author.bot) {
          await interaction.reply({
            ephemeral: true,
            content: "I cannot reply to bot messages.",
          });
          return;
        }

        const cleanContent = interaction.targetMessage.cleanContent.trim();
        const fallbackContent = interaction.targetMessage.content.trim();
        const prompt = cleanContent || fallbackContent;
        if (prompt === "") {
          await interaction.reply({
            ephemeral: true,
            content: "The selected message has no text to reply to.",
          });
          return;
        }

        await interaction.deferReply({ ephemeral: true });

        try {
          const requesterPrefix = `<@${interaction.user.id}> requested me to reply to this message.`;
          const { text: response } = await generateSupportResponse(
            [
              {
                role: "user",
                content: config.replyWithAiContext.requesterPrompt(
                  requesterPrefix,
                  prompt,
                ),
              },
            ],
            config.replyWithAiContext.ai,
          );

          const finalResponse = response.startsWith(requesterPrefix)
            ? response
            : `${requesterPrefix} ${response}`;

          const allowedUserMentions =
            interaction.targetMessage.author.id === interaction.user.id
              ? [interaction.user.id]
              : [interaction.targetMessage.author.id, interaction.user.id];

          await interaction.targetMessage.reply({
            content: finalResponse,
            allowedMentions: {
              users: allowedUserMentions,
              repliedUser: true,
            },
          });

          await interaction.editReply("Reply sent.");
        } catch (error) {
          console.error(error);
          await interaction.editReply(
            "Failed to generate a reply. Please try again later.",
          );
        }

        return;
      } else {
        await interaction.reply("Unknown command");
        return;
      }
    } else {
      return;
    }

    if (trigger === "") return;

    if (trigger === "resolved") {
      if (config.resolved == null) {
        await interaction.reply("This command is not configured.");
        return;
      }

      const channel = interaction.channel;
      if (channel == null || !channel.isThread()) {
        await interaction.reply("This command can only be used in threads.");
        return;
      }

      if (channel.appliedTags.includes(config.resolved.tagId)) {
        await interaction.reply(config.resolved.alreadyResolvedMessage);
        return;
      }

      await interaction.reply(config.resolved.successMessage);
      await channel.setAppliedTags([
        ...channel.appliedTags,
        config.resolved.tagId,
      ]);
      await channel.setLocked(true);
      await channel.setArchived(true, "resolved");

      return;
    }

    if (trigger === "help") {
      if (config.help == null) {
        await interaction.reply("This command is not configured.");
        return;
      }

      const embed = new EmbedBuilder()
        .setColor(bot.accentColor)
        .setTitle(config.help.embedTitle)
        .setDescription(config.help.embedDescription)
        .addFields(
          Object.entries(commandIdRegistry)
            .filter(([, data]) => !data.private)
            .map(([name, data]) => ({
              name: `</${name}:${data.id}>`,
              value: data.description,
              inline: true,
            })),
        );

      await interaction.reply({ embeds: [embed], ephemeral: true });
      return;
    }

    if (trigger === "latest") {
      if (config.latest == null) {
        await interaction.reply("This command is not configured.");
        return;
      }

      const latestRelease = latestReleaseProvider();
      const embed = new EmbedBuilder()
        .setColor(bot.accentColor)
        .setTitle(config.latest.title)
        .setDescription(
          latestRelease == null
            ? "The latest version is not available yet. Try again in a moment."
            : `\`${latestRelease.tag_name}\``,
        );

      await interaction.reply({ embeds: [embed] });
      return;
    }

    const item = commands.find((command) => command.name === trigger);
    if (item == null) {
      await interaction.reply(
        "Something went wrong! I couldn't find that command.",
      );
      return;
    }

    const embed = new EmbedBuilder()
      .setColor(bot.accentColor)
      .setDescription(item.description);

    if (item.url != null) {
      embed.setURL(item.url);
    }

    if (item.docs === true) {
      embed.setTitle(`🔖 ${item.title}`);

      if (config.docsFooter != null) {
        embed.setFooter(config.docsFooter);
      }

      if (item.url != null) {
        embed.addFields([{ name: "Read more", value: item.url }]);
      }
    } else {
      embed.setTitle(`${item.title}`);

      if (item.url != null) {
        embed.addFields([{ name: "Link", value: item.url }]);
      }
    }

    if (item.fields != null) {
      item.fields.forEach((field) => {
        embed.addFields([
          { name: field.key, value: field.value, inline: false },
        ]);
      });
    }

    if (interaction.isChatInputCommand()) {
      const targetUser = interaction.options.getUser("user");
      const message = targetUser == null ? undefined : `<@${targetUser.id}>`;

      await interaction.reply({ content: message, embeds: [embed] });
    } else if (interaction.isMessageContextMenuCommand()) {
      const message = `Requested by <@${interaction.user.id}>`;
      try {
        await interaction.targetMessage.reply({
          content: message,
          embeds: [embed],
        });

        if (isSendHelpFlow) {
          await interaction.editReply({
            content: "Help message sent.",
            components: [],
          });

          setTimeout(() => {
            void interaction.deleteReply().catch(() => {});
          }, 5_000);
        }
      } catch (error) {
        console.error(error);

        if (isSendHelpFlow) {
          await interaction.editReply({
            content:
              "Failed to send the help message. Please check my permissions and try again.",
            components: [],
          });
        } else {
          await interaction.reply({
            ephemeral: true,
            content: "Failed to send the requested message.",
          });
        }
      }
    }
  });
};
