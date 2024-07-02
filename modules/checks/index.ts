import {Client, ColorResolvable, EmbedBuilder, Message} from 'discord.js'

import config, {Checks, MessagePredicate} from './checks.config'
import data from 'data.json'
import tesseract from 'tesseract.js'
import {getMetadata} from "../commands/metadata";

const imageTypes = ['image/png', 'image/jpeg', 'image/webp']

// noinspection JSUnusedGlobalSymbols
export default (client: Client): void => {
  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    let getLink = ''
    let originalLink = ''
    config.checks.every((element: Checks) => {
      const match = element.regex.exec(message.content)
      if (match != null) {
        getLink = element.getLink.replace('{code}', match[1])
        originalLink = match[0]
        return false
      } else {
        return true
      }
    })

    if (!getLink) {
      return
    }

    let response = ''
    try {
      console.log(`Getting upload bin ${getLink}`)
      response = (await (await fetch(getLink)).text())
    } catch (e: any) {
      if (e.response) {
        if (e.response.status === 404) {
          await message.reply({
            embeds: [new EmbedBuilder()
              .setTitle('Invalid Paste!')
              .setColor('#FF0000')
              .setDescription('The paste link you sent in is invalid or expired, please check the link or paste a new one.')
              .setFooter({text: `${originalLink} | Sent by ${message.author.username}`})]
          })
        }
      }

      return
    }

    if (!response) {
      return
    }

    await respondToText(message, response, `${originalLink} | Sent by ${message.author.username}`)
  })

  client.on('messageCreate', async message => {
    if (!message.channel.isTextBased() || message.channel.isDMBased() || message.author.bot) return

    if (message.attachments.size === 0) {
      return
    }

    const attachments = message.attachments
      .filter(attachment => attachment.contentType != null)
      .filter(attachment => imageTypes.includes(attachment.contentType as string))

    if (attachments.size === 0) {
      return
    }

    for (const attachment of attachments.values()) {
      const {data: {text}} = await tesseract.recognize(
        attachment.proxyURL,
        'eng'
      )

      await respondToText(message, text, `Sent by ${message.author.username}`)
    }

    await message.react('👀')
  })
}

function checkMatch(text: string, checks: (RegExp | MessagePredicate)[]) {
  for (const check of checks) {
    if (typeof check === 'function') {
      if (check(text)) {
        return text
      }
    } else {
      const match = check.exec(text)
      if (match != null) {
        return match
      }
    }
  }

  return null
}

async function respondToText(message: Message, text: string, footer: string) {
  for (const test of config.tests) {
    const cause = checkMatch(text, test.checks)
    if (cause == null) {
      continue
    }

    const embed = new EmbedBuilder()
    embed.setTitle(test.title)
    embed.setDescription(test.content)
    if (test.tips != null) {
      embed.addFields(test.tips.map((tip, i) => ({name: `Tip #${i + 1}`, value: tip})))
    }

    if (test.link) {
      embed.addFields({name: 'Read More', value: test.link})
    }

    embed.addFields({name: 'Caused By', value: `\`\`\`${cause}\`\`\``})
    embed.setFooter({text: footer})
    embed.setColor(data.accent_color as ColorResolvable)
    await message.reply({embeds: [embed]})
  }

  // Try parsing SkinsRestorer dump
  if (isJson(text)) {
    try {
      const rawDump = JSON.parse(text)
      const buildInfo = rawDump.buildInfo
      const version = buildInfo.version
      const latestVersion = getMetadata().name

      if (version !== latestVersion) {
        await message.reply({
          embeds: [new EmbedBuilder()
            .setTitle('Outdated SkinsRestorer Version!')
            .setColor('#FF0000')
            .setDescription(`The SkinsRestorer version you're using (${version}) is outdated! Please update to the latest version: ${latestVersion}`)
            .setFooter({text: footer})]
        })
      }
    } catch (e) {
      // Can be ignored, as it's not a SkinsRestorer dump
      console.error(e)
    }
  }
}

function isJson(str: string) {
  try {
    JSON.parse(str);
    return true;
  } catch (e) {
    return false;
  }
}

