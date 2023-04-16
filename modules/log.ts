import { Client } from 'discord.js'
import dateFormat from 'dateformat'
import fs from 'fs'

const getLogFileName = (date: number) => 'logs/' + dateFormat(date, 'yyyy-mm-dd') + '.log'
const getLogFileTime = (date: number) => dateFormat(date, 'hh-MM-ss TT')

// noinspection JSUnusedGlobalSymbols
export default (client: Client) => {
  if (!fs.existsSync('logs')) fs.mkdirSync('logs')

  client.on('messageCreate', (message) => {
    if (!message.channel.type.includes('GUILD') || message.author.bot) return

    if (message.channel.type !== 'GUILD_TEXT') return

    const date = Date.now()
    const log = `${getLogFileTime(date)} [${message.channel.name}] ${message.author.tag}: ${message.content}\n`
    const path = getLogFileName(date)

    fs.appendFile(
      path,
      log,
      function (err) {
        if (err != null) {
          console.log(err)
        }
      }
    )
  })
}
