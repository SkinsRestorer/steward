import { Client, Intents } from 'discord.js'

import config from './config.json'

import fs from 'fs'
const client = new Client({
  intents: [
    Intents.FLAGS.GUILDS,
    Intents.FLAGS.GUILD_MEMBERS,
    Intents.FLAGS.GUILD_PRESENCES,
    Intents.FLAGS.GUILD_MESSAGES,
    Intents.FLAGS.GUILD_MESSAGE_REACTIONS,
    Intents.FLAGS.GUILD_MESSAGE_TYPING,
    Intents.FLAGS.MESSAGE_CONTENT
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
  .map(mod => import(mod))
  .forEach(mod => mod.then((modResolved) => modResolved.default(client)))

await client.login(config.token)
