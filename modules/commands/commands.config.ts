export interface ConfigCommand {
  name: string
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
    name: "wrong-channel",
    title: "This channel is not for support!",
    url: "https://discord.com/channels/186794372468178944/1058044481246605383",
    description: "This channel is not for support for the **SkinsRestorer Minecraft plugin**. Please use the <#1058044481246605383> channel for support, you need to create a post in that channel. You will not receive support in this specific Discord channel."
  },
  {
    name: "docs",
    title: "SkinsRestorer Documentation",
    url: "https://skinsrestorer.net/docs",
    description: "Learn how to use SkinsRestorer and all of its features by reading the wiki.",
    docs: true
  },
  {
    name: "install",
    title: "Installing SkinsRestorer",
    url: "https://skinsrestorer.net/docs/installation",
    description: "You can install SkinsRestorer on Bukkit/Spigot/Paper, BungeeCord, Sponge and Velocity servers. Check the installation guide for more info on setting up SkinsRestorer.",
    docs: true
  },
  {
    name: "proxy-install",
    title: "Network Installation",
    url: "https://skinsrestorer.net/docs/installation",
    description: "If you run a BungeeCord/Velocity network, learn how to correctly setup SkinsRestorer on all server instances (including BungeeCord/Velocity).",
    fields: [
      {
        key: "BungeeCord Installation:",
        value: "https://skinsrestorer.net/docs/installation/bungeecord"
      },
      {
        key: "Velocity Installation:",
        value: "https://skinsrestorer.net/docs/installation/velocity"
      }
    ],
    docs: true
  },
  {
    name: "troubleshooting",
    title: "Troubleshooting",
    url: "https://skinsrestorer.net/docs/troubleshooting",
    description: "Here's a page with some common errors.",
    docs: true
  },
  {
    name: "command-help",
    title: "Command/Permissions Usage",
    url: "https://skinsrestorer.net/docs/configuration/commands-permissions",
    description: "Find all of the available SkinsRestorer commands and permissions on the wiki.",
    docs: true
  },
  {
    name: "api",
    title: "Developer API",
    url: "https://github.com/SkinsRestorer/SkinsRestorerX/wiki/SkinsRestorerAPI",
    description: "Learn how to use the SkinsRestorer API in your project.",
    fields: [
      {
        key: "Example usages",
        value: "https://github.com/SkinsRestorer/SkinsRestorerAPIExample"
      },
      {
        key: "Plugin messaging channel",
        value: "https://github.com/SkinsRestorer/SRPluginMessagingChannelExample"
      },
      {
        key: "Javadocs",
        value: "https://docs.skinsrestorer.net"
      }
    ],
    docs: true
  },
  {
    name: "config",
    title: "SkinsRestorer Configuration",
    url: "https://skinsrestorer.net/docs/configuration",
    description: "Learn what each of the config options are for.",
    docs: true
  },
  {
    name: "storage",
    title: "SkinsRestorer Data Storage",
    url: "https://skinsrestorer.net/docs/development/storage",
    description: "Here is how data storage works in SkinsRestorer.",
    docs: true
  },
  {
    name: "launcher-issues",
    title: "Launcher skin issues",
    url: "https://skinsrestorer.net/docs/troubleshooting/launcher-issues",
    description: "Here is how to fix skin issues with some launchers.",
    docs: true
  },
  {
    name: "downloads",
    title: "Downloads",
    url: "https://www.spigotmc.org/resources/skinsrestorer.2124/",
    description: "You can download SkinsRestorer for Bukkit/Spigot/Paper, BungeeCord, Sponge and Velocity.",
    fields: [
      {
        key: "Dev downloads",
        value: "https://ci.codemc.io/job/SkinsRestorer/job/SkinsRestorerX-DEV/"
      }
    ]
  },
  {
    name: "crowdin",
    title: "Translating SkinsRestorer",
    url: "https://translate.skinsrestorer.net",
    description: "Translations for SkinsRestorer are managed on Crowdin. Any contributions are very welcome!"
  },
  {
    name: "forge",
    title: "Cauldron, Thermos, Forge + Bukkit Hacks, SpongeForge",
    description: "We don't support those! They are hacky and do not work with our plugin most of the time! Try Skinport or Offlineskin",
    docs: true
  },
  {
    name: "color-codes",
    title: "Colour Codes",
    description: "A helpful list of all colour codes that you can use.",
    fields: [
      {
        key: "Colours",
        value: "https://wiki.ess3.net/mc/"
      }
    ]
  },
  {
    name: "not-working",
    title: "Please tell us what's going on!",
    description: "We really would absolutely love to help you out! However, telling us that it isn't working wastes everyone's time. Please, just **describe the issue you're having clearly** and with as much detail as possible, and **send any relevant screenshots** of whatever problems you're having.",
    fields: [
      {
        key: "For sending us Console Errors:",
        value: "https://pastes.dev/"
      }
    ]
  },
  {
    name: "issue-tracker",
    title: "Suggestions and Bug Reports",
    description: "If you would like to request a feature for SkinsRestorer, or report a bug, feel free to open an issue on GitHub!",
    fields: [
      {
        key: "Issue Tracker:",
        value: "https://github.com/SkinsRestorer/SkinsRestorerX/issues"
      }
    ]
  },
  {
    name: "server-info",
    title: "Please take a screenshot!",
    description: "Seeing a screenshot makes everything so much easier!",
    fields: [
      {
        key: "For SkinsRestorer info:",
        value: "`/sr status`"
      },
      {
        key: "For server info:",
        value: "`/version`"
      }
    ]
  },
  {
    name: "paste-it",
    title: "Please use a paste service!",
    description: "Seeing a paste of the problem makes everything so much easier! Use https://pastes.dev/ for easy pasting!",
    fields: [
      {
        key: "For console errors:",
        value: "Paste any relevant segments of the console log. If it's a startup error, this includes the entire startup log!"
      },
      {
        key: "Other errors:",
        value: "Paste the entire SkinsRestorer config file (passwords removed) as well as any other relevant files!"
      }
    ]
  },
  {
    name: "just-ask",
    title: "Please ask your question!",
    description: "Please ask the question you have. Don't ask to ask, or ask to DM someone. There are people here to help you, but we need to know what to help you with, so please just ask the question you want to in as much detail as possible!",
    fields: [
      {
        key: "Or, try here first:",
        value: "https://skinsrestorer.net/docs"
      },
      {
        key: "Why shouldn't I ask to ask?",
        value: "https://sol.gfxile.net/dontask.html"
      }
    ]
  },
  {
    name: "no-wildcard",
    title: "Wildcard issues",
    description: "Some plugins are created in a way which results in odd behaviour when the root '*' wildcard is used.",
    fields: [
      {
        key: "More information:",
        value: "https://nucleuspowered.org/docs/nowildcard.html"
      }
    ]
  }
]