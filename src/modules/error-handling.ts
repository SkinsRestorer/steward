import { Client } from 'discord.js'

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  client.on('error', console.error)
}
