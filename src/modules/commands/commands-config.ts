export interface ConfigCommand {
  name: string
  cmdDescription: string
  url?: string
  title: string
  description: string
  docs?: boolean
  fields?: CommandField[]
}

export interface CommandField {
  key: string
  value: string
}

export const configCommands: ConfigCommand[] = [
  {
    name: 'wrong-channel',
    cmdDescription: 'Send a message that the channel is wrong',
    title: 'This channel is not for support!',
    url: 'https://discord.com/channels/186794372468178944/1058044481246605383',
    description: 'This channel is not for support for the **SkinsRestorer Minecraft plugin**. Please use the <#1058044481246605383> channel for support, you need to create a post in that channel. You will not receive support in this specific Discord channel.'
  },
  {
    name: 'docs',
    cmdDescription: 'Send a link to the docs',
    title: 'SkinsRestorer Documentation',
    url: 'https://skinsrestorer.net/docs',
    description: 'Learn how to use SkinsRestorer and all of its features by reading the wiki.',
    docs: true
  },
  {
    name: 'install',
    cmdDescription: 'Send a message with a link to the installation guide',
    title: 'Installing SkinsRestorer',
    url: 'https://skinsrestorer.net/docs/installation',
    description: 'You can install SkinsRestorer on Bukkit/Spigot/Paper, BungeeCord, Sponge and Velocity servers. Check the installation guide for more info on setting up SkinsRestorer.',
    docs: true
  },
  {
    name: 'proxy-install',
    cmdDescription: 'Send a link to the proxy installation guide',
    title: 'Network Installation',
    url: 'https://skinsrestorer.net/docs/installation',
    description: 'If you run a BungeeCord/Velocity network, learn how to correctly setup SkinsRestorer on all server instances (including BungeeCord/Velocity).',
    fields: [
      {
        key: 'BungeeCord Installation:',
        value: 'https://skinsrestorer.net/docs/installation/bungeecord'
      },
      {
        key: 'Velocity Installation:',
        value: 'https://skinsrestorer.net/docs/installation/velocity'
      }
    ],
    docs: true
  },
  {
    name: 'troubleshooting',
    cmdDescription: 'Send a link to the troubleshooting guide',
    title: 'Troubleshooting',
    url: 'https://skinsrestorer.net/docs/troubleshooting',
    description: "Here's a page with some common errors.",
    docs: true
  },
  {
    name: 'command-help',
    cmdDescription: 'Send a link to the command help page',
    title: 'Command/Permissions Usage',
    url: 'https://skinsrestorer.net/docs/configuration/commands-permissions',
    description: 'Find all of the available SkinsRestorer commands and permissions on the wiki.',
    docs: true
  },
  {
    name: 'api',
    cmdDescription: 'Send a link to the API page',
    title: 'Developer API',
    url: 'https://github.com/SkinsRestorer/SkinsRestorer/wiki/SkinsRestorerAPI',
    description: 'Learn how to use the SkinsRestorer API in your project.',
    fields: [
      {
        key: 'Example usages',
        value: 'https://github.com/SkinsRestorer/SkinsRestorerAPIExample'
      },
      {
        key: 'Plugin messaging channel',
        value: 'https://github.com/SkinsRestorer/SRPluginMessagingChannelExample'
      },
      {
        key: 'Javadocs',
        value: 'https://docs.skinsrestorer.net'
      }
    ],
    docs: true
  },
  {
    name: 'config',
    cmdDescription: 'Send a link to the config page',
    title: 'SkinsRestorer Configuration',
    url: 'https://skinsrestorer.net/docs/configuration',
    description: 'Learn what each of the config options are for.',
    docs: true
  },
  {
    name: 'storage',
    cmdDescription: 'Send a link to the storage page',
    title: 'SkinsRestorer Data Storage',
    url: 'https://skinsrestorer.net/docs/development/storage',
    description: 'Here is how data storage works in SkinsRestorer.',
    docs: true
  },
  {
    name: 'launcher-issues',
    cmdDescription: 'Send a link to the launcher issues page',
    title: 'Launcher skin issues',
    url: 'https://skinsrestorer.net/docs/troubleshooting/launcher-issues',
    description: 'Here is how to fix skin issues with some launchers.',
    docs: true
  },
  {
    name: 'tlauncher',
    cmdDescription: 'Explain how to fix TLauncher issues',
    title: 'TLauncher skin issues',
    description: 'TLauncher is malware. If you still want to use it, you have to know that its own skin system breaks SkinsRestorer. It simply ignores the skin set by SkinsRestorer in favor of its own skin system. You have to disable it in the TLauncher settings. A link to the documentation is below.',
    url: 'https://skinsrestorer.net/docs/troubleshooting/launcher-issues#tlauncher',
    docs: true
  },
  {
    name: 'auto-update',
    cmdDescription: 'Explain what auto update is for',
    title: 'Why does SkinsRestorer auto-update?',
    description: 'Auto-updating is a feature that allows SkinsRestorer to automatically update to the latest version. This is to ensure that you are always running the latest version of the plugin, which may contain important bug fixes and new features. The latest version always supports older versions of Minecraft.',
    url: 'https://skinsrestorer.net/docs/configuration/auto-update',
    docs: true
  },
  {
    name: 'downloads',
    cmdDescription: 'Send a link to the downloads page',
    title: 'Downloads',
    url: 'https://modrinth.com/plugin/skinsrestorer',
    description: 'You can download SkinsRestorer for Bukkit/Spigot/Paper, BungeeCord, Sponge and Velocity.',
    fields: [
      {
        key: 'Dev downloads',
        value: 'https://ci.codemc.io/job/SkinsRestorer/job/SkinsRestorer-DEV/'
      }
    ]
  },
  {
    name: 'crowdin',
    cmdDescription: 'Send a link to the Crowdin page',
    title: 'Translating SkinsRestorer',
    url: 'https://translate.skinsrestorer.net',
    description: 'Translations for SkinsRestorer are managed on Crowdin. Any contributions are very welcome!'
  },
  {
    name: 'forge',
    cmdDescription: 'Send a message that Forge is not supported',
    title: 'Cauldron, Thermos, Forge + Bukkit Hacks, SpongeForge',
    description: "We don't support those! They are hacky and do not work with our plugin most of the time! Try Skinport or Offlineskin",
    docs: true
  },
  {
    name: 'color-codes',
    cmdDescription: 'Send a link to a page with color codes',
    title: 'Colour Codes',
    description: 'A helpful list of all colour codes that you can use.',
    fields: [
      {
        key: 'Colours',
        value: 'https://wiki.ess3.net/mc/'
      }
    ]
  },
  {
    name: 'not-working',
    cmdDescription: 'Send a message that the plugin is not working',
    title: "Please tell us what's going on!",
    description: "We really would absolutely love to help you out! However, telling us that it isn't working wastes everyone's time. Please, just **describe the issue you're having clearly** and with as much detail as possible, and **send any relevant screenshots** of whatever problems you're having.",
    fields: [
      {
        key: 'For sending us Console Errors:',
        value: 'https://pastes.dev/'
      }
    ]
  },
  {
    name: 'issue-tracker',
    cmdDescription: 'Send a link to the issue tracker',
    title: 'Suggestions and Bug Reports',
    description: 'If you would like to request a feature for SkinsRestorer, or report a bug, feel free to open an issue on GitHub!',
    fields: [
      {
        key: 'Issue Tracker:',
        value: 'https://github.com/SkinsRestorer/SkinsRestorer/issues'
      }
    ]
  },
  {
    name: 'server-info',
    cmdDescription: 'Send a message with server info',
    title: 'Please take a screenshot!',
    description: 'Seeing a screenshot makes everything so much easier!',
    fields: [
      {
        key: 'For SkinsRestorer info:',
        value: '`/sr status`'
      },
      {
        key: 'For server info:',
        value: '`/version`'
      }
    ]
  },
  {
    name: 'send-logs',
    cmdDescription: 'Send a message with info to send logs',
    title: 'Please send us your server logs!',
    description: 'Send us your entire console log. Use a service like https://mclo.gs/ to paste the logs.',
    fields: [
      {
        key: 'Where to find logs?',
        value: 'In your server console or in the `./logs/latest.log` file of your server.'
      },
      {
        key: 'Why do we need logs?',
        value: 'Error message are most useful to us when they are in the context of the rest of the log and give us info about what part of the plugin is causing the issue.'
      },
      {
        key: 'Do not leak private player IPs!',
        value: 'Services like https://mclo.gs/ hide player IPs from the uploaded logs, so you can safely share them. Make sure you manually remove them if you use a different service.'
      }
    ]
  },
  {
    name: 'just-ask',
    cmdDescription: 'Send a message that the user should just ask their question',
    title: 'Please ask your question!',
    description: "Please ask the question you have. Don't ask to ask, or ask to DM someone. There are people here to help you, but we need to know what to help you with, so please just ask the question you want to in as much detail as possible!",
    fields: [
      {
        key: 'Or, try here first:',
        value: 'https://skinsrestorer.net/docs'
      },
      {
        key: "Why shouldn't I ask to ask?",
        value: 'https://sol.gfxile.net/dontask.html'
      }
    ]
  },
  {
    name: 'no-wildcard',
    cmdDescription: 'Send a message that the user should not use the wildcard',
    title: 'Wildcard issues',
    description: "Some plugins are created in a way which results in odd behaviour when the root '*' wildcard is used.",
    fields: [
      {
        key: 'More information:',
        value: 'https://nucleuspowered.org/docs/nowildcard.html'
      }
    ]
  },
  {
    name: 'proxy-mode',
    cmdDescription: 'Send a message that explains Proxy Mode',
    title: 'SkinsRestorer Proxy Mode',
    description: 'SkinsRestorer Proxy Mode is one of the modes that SkinsRestorer can run in. It is used for servers in BungeeCord/Velocity networks.',
    fields: [
      {
        key: 'What does it do?',
        value: 'In this mode SkinsRestorer acts as a receiver of skin data from the proxy. By itself the plugin does not store any data on the server and only acts as a middleman between the proxy and the player.'
      },
      {
        key: 'What does it differently?',
        value: 'SkinsRestorer no longer stores data, does not have a API, and does not register commands. It will listen for messages for applying a skin and opening the skin GUI on a plugin messaging channel.'
      },
      {
        key: 'How do I use it?',
        value: 'SkinsRestorer Proxy Mode is automatically detected when the server is configured to only accept connections from proxies. Usually this is configured in `spigot.yml`, `paper.yml`, or `config/paper-global.yml`.'
      },
      {
        key: "What if I don't want to use it?",
        value: "If you don't put SkinsRestorer on your backend servers, your proxy will no longer be able to refresh your players skin without rejoining and the skin GUI will not work."
      },
      {
        key: 'What other options do I have?',
        value: 'You can use SkinsRestorer in standalone mode, which is the default mode, in that case you should not put the plugin on your proxy and you need to manually link your backend servers via MySQL.'
      }
    ]
  },
  {
    name: 'sr-dump',
    cmdDescription: 'Send a message to run /sr dump',
    title: 'Please run `/sr dump` in-game or in the console',
    description: 'You will be sent a link by the server with a dump of your SkinsRestorer configuration and system information. Please send the link in this channel.'
  }
]
