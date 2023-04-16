import {Client, GatewayIntentBits} from 'discord.js'

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
  ]
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
    .forEach(async mod => await mod.then((modResolved) => modResolved.default(client)))

await client.login(config.token)
