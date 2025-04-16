import { Client, GatewayIntentBits, ActivityType } from 'discord.js'
import fs from 'fs'
import "dotenv";
import { fileURLToPath } from 'url';
import path from 'path';

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
  console.log(`Logged in as ${client.user?.tag ?? 'unknown'}!`)
})

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
fs.readdirSync(path.resolve(__dirname, './modules'))
  .map(mod => {
    console.log('Loading module: ' + mod)
    return `./modules/${mod}`
  })
  .map(async mod => await import(mod))
  .forEach(async mod => {
    const modResolved = await mod
    await modResolved.default(client)
  })

await client.login(process.env.DISCORD_TOKEN!)
