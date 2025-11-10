import {
  ActionRowBuilder,
  type ActionRowData,
  ApplicationCommandType,
  type Client,
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
} from "discord.js";
import type { RESTPutAPIApplicationCommandsResult } from "discord-api-types/v10";
import data from "@/data.json" with { type: "json" };
import { generateSupportResponse } from "@/lib/ai";
import { type ConfigCommand, configCommands } from "./commands-config";
import { getMetadata } from "./metadata";

const commands: ConfigCommand[] = configCommands.sort((a, b) => {
  if (a.name < b.name) return -1;
  if (a.name > b.name) return 1;
  return 0;
});

const slashApiCommands: RESTPostAPIApplicationCommandsJSONBody[] = [
  new SlashCommandBuilder()
    .setName("help")
    .setDescription("Show Steward help")
    .addUserOption((option) =>
      option
        .setName("user")
        .setDescription("Mention a specific user with the command"),
    )
    .toJSON(),
  new SlashCommandBuilder()
    .setName("latest")
    .setDescription("Show latest version on GitHub")
    .addUserOption((option) =>
      option
        .setName("user")
        .setDescription("Mention a specific user with the command"),
    )
    .toJSON(),
  new SlashCommandBuilder()
    .setName("resolved")
    .setDescription("Moderator command to mark a forum post as resolved")
    .setDefaultMemberPermissions(PermissionFlagsBits.ManageThreads)
    .setContexts([InteractionContextType.Guild])
    .toJSON(),
];

for (const command of commands) {
  const slashCommand = new SlashCommandBuilder()
    .setName(command.name)
    .setDescription(command.cmdDescription)
    .addUserOption((option) =>
      option
        .setName("user")
        .setDescription("Mention a specific user with the command"),
    );

  slashApiCommands.push(slashCommand.toJSON());
}

const sendHelpContext = new ContextMenuCommandBuilder()
  .setName("Send Help")
  .setType(ApplicationCommandType.Message)
  .toJSON();
slashApiCommands.push(sendHelpContext);

const sendSupportContext = new ContextMenuCommandBuilder()
  .setName("Send to Support Forum")
  .setType(ApplicationCommandType.Message)
  .toJSON();
slashApiCommands.push(sendSupportContext);

const replyWithAiContext = new ContextMenuCommandBuilder()
  .setName("Reply with AI")
  .setType(ApplicationCommandType.Message)
  .toJSON();
slashApiCommands.push(replyWithAiContext);

interface CommandData {
  id: string;
  description: string;
  private: boolean;
}

export const commandIdRegistry: Record<string, CommandData> = {};

// noinspection JSUnusedGlobalSymbols
export default async (client: Client): Promise<void> => {
  const discordToken = process.env.DISCORD_TOKEN;
  if (discordToken == null || discordToken === "") {
    throw new Error("DISCORD_TOKEN environment variable is not defined");
  }

  const rest = new REST().setToken(discordToken);

  console.log(
    `Started refreshing ${slashApiCommands.length} application (/) commands.`,
  );

  // The put method is used to fully refresh all commands in the guild with the current set
  const responseData = (await rest.put(
    Routes.applicationCommands(data.clientId),
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
    `Successfully reloaded ${responseData.length} application (/) commands.`,
  );

  client.on("interactionCreate", async (interaction) => {
    let isSendHelpFlow = false;
    if (
      !interaction.isChatInputCommand() &&
      !interaction.isMessageContextMenuCommand()
    )
      return;

    // Get the trigger
    let trigger: string;
    if (interaction.isChatInputCommand()) {
      trigger = interaction.commandName;
    } else if (interaction.isMessageContextMenuCommand()) {
      if (interaction.commandName === sendHelpContext.name) {
        isSendHelpFlow = true;
        const customId = `help-selection-${Date.now()}`;
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
      } else if (interaction.commandName === sendSupportContext.name) {
        const embed = new EmbedBuilder()
          .setColor(data.accent_color as ColorResolvable)
          .setTitle("This channel is not for support!")
          .setDescription(
            "This channel is not for support for the **SkinsRestorer Minecraft plugin**. Please use the <#1058044481246605383> channel for support, you need to create a post in that channel. You will not receive support in this specific Discord channel.",
          )
          .setURL(
            "https://discord.com/channels/186794372468178944/1058044481246605383",
          );

        const message = `<@${interaction.targetMessage.author.id}> Requested by <@${interaction.user.id}>`;

        await interaction.targetMessage.reply({
          content: message,
          embeds: [embed],
        });

        return;
      } else if (interaction.commandName === replyWithAiContext.name) {
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
          const response = await generateSupportResponse([
            {
              role: "user",
              content: `Begin your response with "${requesterPrefix}" exactly before offering help. Here is the message you must answer:\n${prompt}`,
            },
          ]);

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

    // Ignore if trigger is blank
    if (trigger === "") return;

    if (trigger === "resolved") {
      const channel = interaction.channel;
      if (channel == null || !channel.isThread()) {
        await interaction.reply("This command can only be used in threads.");
        return;
      }

      if (channel.appliedTags.includes(data.resolvedTag)) {
        await interaction.reply("This thread is already marked as resolved.");
        return;
      }

      // Send a message before locking the thread, so we can still reply to it
      await interaction.reply(
        "This thread has been marked as resolved, locked and archived. Thank you for using SkinsRestorer! For any future issues, please create a new post.",
      );
      await channel.setAppliedTags([...channel.appliedTags, data.resolvedTag]);
      await channel.setLocked(true);
      await channel.setArchived(true, "resolved");

      return;
    }

    if (trigger === "help") {
      const embed = new EmbedBuilder()
        .setColor(data.accent_color as ColorResolvable)
        .setTitle("Steward help")
        .setDescription(
          "Hi! :wave: I am Steward. Here to help out at SkinsRestorer. The code for steward can be [found on GitHub](https://github.com/SkinsRestorer/steward)",
        )
        .addFields(
          Object.entries(commandIdRegistry)
            .filter(([, data]) => !data.private)
            .map(([name, data]) => {
              return {
                name: `</${name}:${data.id}>`,
                value: data.description,
                inline: true,
              };
            }),
        );

      await interaction.reply({ embeds: [embed], ephemeral: true });
      return;
    }

    if (trigger === "latest") {
      const embed = new EmbedBuilder()
        .setColor(data.accent_color as ColorResolvable)
        .setTitle("Latest version")
        .setDescription(`\`${getMetadata().tag_name}\``);

      await interaction.reply({ embeds: [embed] });
      return;
    }

    // Check if command name exists
    const item = commands.find((command) => {
      return command.name === trigger;
    });

    // If no command found, throw an error
    if (item === null || item === undefined) {
      await interaction.reply(
        "Something went wrong! I couldn't find that command.",
      );
      return;
    }

    const embed = new EmbedBuilder();

    // Begin formatting the command embed
    embed
      .setColor(data.accent_color as ColorResolvable)
      .setDescription(item.description);

    if (item.url != null) {
      embed.setURL(item.url);
    }

    if (item.docs === true) {
      embed.setTitle(`🔖 ${item.title}`).setFooter({
        text: "SkinsRestorer documentation",
        iconURL: "https://skinsrestorer.net/logo.png",
      });

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
      let message: string | undefined;
      if (targetUser != null) {
        message = `<@${targetUser.id}>`;
      }

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
