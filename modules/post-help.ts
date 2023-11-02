import {ChannelType, Client, EmbedBuilder} from 'discord.js'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('threadCreate', async thread => {
    if (thread.parent?.type !== ChannelType.GuildForum) return

    if (!thread.joinable) return

    await thread.join()

    const embed = new EmbedBuilder()

    embed.setTitle('Welcome to SkinsRestorer Support!')
    embed.setDescription('This is a automatic message with tips on how to help us to help you better.')
    embed.setColor('#F8E839')
    embed.addFields([
      {
        name: 'Make sure you are using the latest version of SkinsRestorer',
        value: 'You can download the latest version [from our website](https://skinsrestorer.net). We do not support old versions. Often times, your issue is already fixed in the latest version.'
      },
      {
        name: 'Read our documentation',
        value: 'We have a [documentation](https://skinsrestorer.net/docs) on our website. It contains a lot of useful information, including how to setup SkinsRestorer.'
      },
      {
        name: 'Search the issue tracker',
        value: 'We have a [issue tracker](https://github.com/SkinsRestorer/SkinsRestorerX/issues) on GitHub. You can search for your issue there. If you find it, you can subscribe to it to get notified when it is fixed.'
      },
      {
        name: 'Give us more information!',
        value: 'When you create a thread, please include as much information as possible. This includes:\n' +
          '- Your SkinsRestorer version\n' +
          '- Your server version\n' +
          '- Your server software (Spigot, Paper, etc.)\n' +
          '- Your server log\n' +
          '- The link created by running `/sr dump`\n' +
          '- Any other relevant information\n' +
          'You can use [pastes.dev](https://pastes.dev) to share files. If you do not include this information, we will ask for it and your thread will be closed until you provide it.'
      },
      {
        name: 'Be patient',
        value: 'We are a small team and cannot always respond immediately. If we do not respond within a few days, feel free to ping us.'
      }
    ])
    embed.setFooter({text: 'If you have any questions, feel free to ask them here. We are a small team, if you would like to support us, you can donate [via PayPal](https://skinsrestorer.net/donate).'})

    await thread.send({embeds: [embed]})
  })
}
