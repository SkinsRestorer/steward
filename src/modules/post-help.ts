import {
  ActionRowBuilder,
  ButtonBuilder,
  ButtonStyle,
  ChannelType,
  type Client,
  EmbedBuilder,
} from "discord.js";

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on("threadCreate", async (thread) => {
    if (thread.parent?.type !== ChannelType.GuildForum) return;
    if (!thread.joinable) return;

    await thread.join();

    const originalMessage = await thread.fetchStarterMessage();
    if (!originalMessage) return;

    const supportGptEmbed = new EmbedBuilder()
      .setColor("#F8E839")
      .setTitle("Need quick SkinsRestorer help?")
      .setDescription(
        [
          "Meet the **SkinsRestorer Support GPT** — our personal AI assistant trained on SkinsRestorer knowledge and docs.",
          "A GPT is a conversational AI you can chat with like a teammate; it stays online 24/7 to guide you through setup, config tweaks, and smaller issues with detailed answers.",
          "If you still need us, drop the specifics of your problem here and we'll follow up as soon as we can!",
        ].join("\n\n"),
      );

    const prioritySupportEmbed = new EmbedBuilder()
      .setColor("#F8E839")
      .setTitle("Need private priority support?")
      .setDescription(
        [
          "We now offer a **Priority Support** membership for users who want private, faster help from the SkinsRestorer team.",
          "The membership costs **5 EUR/month** and is a good fit if you want one-on-one troubleshooting instead of waiting in the public forum.",
          "You can compare the available options on our website or join directly through Ko-fi below.",
        ].join("\n\n"),
      );

    const supportLinksRow = new ActionRowBuilder<ButtonBuilder>().addComponents(
      new ButtonBuilder()
        .setLabel("🤖 Open Support GPT")
        .setStyle(ButtonStyle.Link)
        .setURL(
          "https://chatgpt.com/g/g-68f7a885f5688191b9a05f812f4ccf43-skinsrestorer-support-gpt",
        ),
      new ButtonBuilder()
        .setLabel("📚 Open Documentation")
        .setStyle(ButtonStyle.Link)
        .setURL("https://skinsrestorer.net/docs"),
    );

    const membershipLinksRow =
      new ActionRowBuilder<ButtonBuilder>().addComponents(
        new ButtonBuilder()
          .setLabel("💳 View Pricing")
          .setStyle(ButtonStyle.Link)
          .setURL("https://skinsrestorer.net/pricing"),
        new ButtonBuilder()
          .setLabel("☕ Join via Ko-fi")
          .setStyle(ButtonStyle.Link)
          .setURL("https://ko-fi.com/skinsrestorer/tiers"),
      );

    await originalMessage.reply({
      embeds: [supportGptEmbed, prioritySupportEmbed],
      components: [supportLinksRow, membershipLinksRow],
    });
  });
};
