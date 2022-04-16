import { Client } from 'discord.js'

import dateFormat from 'dateformat'

import fs from 'fs'

const getLogFileName = (date: number) => 'logs/' + dateFormat(date, 'yyyy-mm-dd') + '.log'
const getLogFileTime = (date: number) => dateFormat(date, 'hh-MM-ss TT')

export default (client: Client) => {
  client.on('message', (message) => {
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
