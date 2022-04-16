import { Client, Intents } from 'discord.js'

import config from './config.json' assert { type: "json" }

import fs from 'fs'
const client = new Client({ intents: [Intents.FLAGS.GUILDS] })

client.on('ready', () => {
  console.log(`Logged in as ${client.user?.tag}!`)
})

fs.readdirSync('modules')
  .map(mod => {
    console.log('Loading module: ' + mod)
    return `./modules/${mod}`
  })
  .map(mod => import(mod))
  .forEach(mod => mod.then((modResolved) => modResolved(client)))

client.login(config.token)
