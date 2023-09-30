export interface Checks {
  regex: RegExp
  getLink: string
}

export interface Tests {
  checks: RegExp[]
  title: string
  content: string
  tips?: string[]
  link?: string
}

export interface ChecksConfig {
  checks: Checks[]
  tests: Tests[]
}

const checksConfig: ChecksConfig = {
  checks: [
    { regex: /https?:\/\/hastebin\.com\/(\w+)(?:\.\w+)?/g, getLink: 'https://hastebin.com/raw/{code}' },
    { regex: /https?:\/\/hasteb\.in\/(\w+)(?:\.\w+)?/g, getLink: 'https://hasteb.in/raw/{code}' },
    { regex: /https?:\/\/paste\.helpch\.at\/(\w+)(?:\.\w+)?/g, getLink: 'https://paste.helpch.at/raw/{code}' },
    { regex: /https?:\/\/bytebin\.lucko\.me\/(\w+)/g, getLink: 'https://bytebin.lucko.me/{code}' },
    { regex: /https?:\/\/pastes\.dev\/(\w+)/g, getLink: 'https://bytebin.lucko.me/{code}' },
    { regex: /https?:\/\/paste\.lucko\.me\/(\w+)(?:\.\w+)?/g, getLink: 'https://paste.lucko.me/raw/{code}' },
    { regex: /https?:\/\/pastebin\.com\/(\w+)(?:\.\w+)?/g, getLink: 'https://pastebin.com/raw/{code}' },
    { regex: /https?:\/\/gist\.github\.com\/(\w+\/\w+)(?:\.\w+\/\w+)?/g, getLink: 'https://gist.github.com/{code}/raw/' },
    { regex: /https?:\/\/gitlab\.com\/snippets\/(\w+)(?:\.\w+)?/g, getLink: 'https://gitlab.com/snippets/{code}/raw' }
  ],

  tests: [
    {
      checks: [/SkinsRestorerAPI is not initialized yet/g],
      title: 'SkinsRestorerAPI is not initialized yet',
      content: 'This error occurs when a third-party plugin tries to access SkinsRestorerAPI before it is fully loaded. This is a bug in the third-party plugin, and should be reported to the plugin developer.',
      tips: [
        "Make sure SkinsRestorer is installed and enabled. There may have been a startup error that prevented SkinsRestorer from loading.",
        'Your plugin may be loading before SkinsRestorer. To load your plugin after SkinsRestorer, add `softdepend: [ "SkinsRestorer" ]` to your plugin.yml file.'
      ],
      link: 'https://skinsrestorer.net/docs/development/api#add-skinsrestorer-as-a-dependency'
    }
  ]
}

export default checksConfig
