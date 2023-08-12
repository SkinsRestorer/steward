import { Client, GatewayIntentBits, ActivityType } from 'discord.js'

import config from './config.json'

import fs from 'fs'

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMembers,
    GatewayIntentBits.GuildPresences,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.GuildMessageReactions,
    GatewayIntentBits.GuildMessageTyping,
    GatewayIntentBits.MessageContent
  ],
  presence: {
    status: 'online',
    activities: [
      {
        name: 'SR Discord',
        type: ActivityType.Watching
      }
    ]
  }
})

client.on('ready', () => {
  console.log(`Logged in as ${client.user?.tag}!`)
})

fs.readdirSync('modules')
  .map(mod => {
    console.log('Loading module: ' + mod)
    return `./modules/${mod}`
  })
  .map(async mod => await import(mod))
  .forEach(async mod => {
    const modResolved = await mod
    await modResolved.default(client)
  })

await client.login(config.token)
