export interface Checks {
  regex: RegExp
  getLink: string
}

export interface Tests {
  checks: RegExp[]
  title: string
  link: string
}

export interface ChecksConfig {
  checks: Checks[]
  tests: Tests[]
}

const checksConfig: ChecksConfig = {
  checks: [
    {regex: /https?:\/\/hastebin\.com\/(\w+)(?:\.\w+)?/g, getLink: 'https://hastebin.com/raw/{code}'},
    {regex: /https?:\/\/hasteb\.in\/(\w+)(?:\.\w+)?/g, getLink: 'https://hasteb.in/raw/{code}'},
    {regex: /https?:\/\/paste\.helpch\.at\/(\w+)(?:\.\w+)?/g, getLink: 'https://paste.helpch.at/raw/{code}'},
    {regex: /https?:\/\/bytebin\.lucko\.me\/(\w+)/g, getLink: 'https://bytebin.lucko.me/{code}'},
    {regex: /https?:\/\/paste\.lucko\.me\/(\w+)(?:\.\w+)?/g, getLink: 'https://paste.lucko.me/raw/{code}'},
    {regex: /https?:\/\/pastebin\.com\/(\w+)(?:\.\w+)?/g, getLink: 'https://pastebin.com/raw/{code}'},
    {regex: /https?:\/\/gist\.github\.com\/(\w+\/\w+)(?:\.\w+\/\w+)?/g, getLink: 'https://gist.github.com/{code}/raw/'},
    {regex: /https?:\/\/gitlab\.com\/snippets\/(\w+)(?:\.\w+)?/g, getLink: 'https://gitlab.com/snippets/{code}/raw'}
  ],

  tests: []
}

export default checksConfig
