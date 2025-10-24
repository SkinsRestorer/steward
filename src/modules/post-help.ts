import { ChannelType, type Client, EmbedBuilder } from "discord.js";

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on("threadCreate", async (thread) => {
    if (thread.parent?.type !== ChannelType.GuildForum) return;
    if (!thread.joinable) return;

    await thread.join();

    const originalMessage = await thread.fetchStarterMessage();
    if (!originalMessage) return;

    const embed = new EmbedBuilder()
      .setColor("#F8E839")
      .setTitle("Need quick SkinsRestorer help?")
      .setDescription(
        [
          "Meet the **SkinsRestorer Support GPT** — our personal AI assistant trained on SkinsRestorer knowledge and docs.",
          "A GPT is a conversational AI you can chat with like a teammate; it stays online 24/7 to guide you through setup, config tweaks, and smaller issues with detailed answers.",
          "Start a chat any time at https://chatgpt.com/g/g-68f7a885f5688191b9a05f812f4ccf43-skinsrestorer-support-gpt. If you still need us, drop the specifics of your problem here and we'll follow up as soon as we can!",
        ].join("\n\n"),
      );

    await originalMessage.reply({ embeds: [embed] });
  });
};
