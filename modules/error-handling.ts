import { Client } from 'discord.js'

export default (client: Client) => {
  client.on('error', console.error)
}
