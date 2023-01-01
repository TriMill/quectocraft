# Quectocraft Plugin API

Quectocraft plugins are written in Lua. Examples can be seen in the [plugins directory](plugins). Plugins can either be a single Lua file or a directory containing a file named `main.lua`.

## Plugin table

All information about a plugin is stored in a table, which must be returned at the end of the plugin. This table contains both information about the plugin and functions that act as event handlers.

| Field              | Description                                                                                                                                      |
|--------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| `id`               | The plugin's ID. This should consist of lowercase letters and underscores only.                                                                  |
| `name`             | The plugin's human-readable name.                                                                                                                |
| `description`      | The plugin's description.                                                                                                                        |
| `authors`          | A list of the plugin's authors.                                                                                                                  |
| `version`          | The plugin's version (semantic versioning encouraged).                                                                                           |
| `init`             | Called when the plugin is initialized. The `server` table is available at this point.                                                            |
| `registerCommands` | Called to register the plugin's commands. Arguments: a `registry` table.                                                                         |
| `playerJoin`       | Called when a player joins. Arguments: the player's name, the player's UUID.                                                                     |
| `playerLeave`      | Called when a player leaves. Arguments: the player's name, the player's UUID.                                                                    |
| `chatMessage`      | Called when a player sends a chat message. Arguments: the message, the player's name and UUID.                                                   |
| `pluginMessage`    | Called when a client sends a [plugin message](https://wiki.vg/Plugin_channels). Arguments: the channel, the message, the player's name and UUID. |
| `command`          | Called when a player runs a command. Arguments: the command, the arguments, the player's name and UUID.                                          |

## The `server` table

The `server` table is used to interact with the server. It has the following fields:

| Field               | Description                                                                                                                        |
|---------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `players`           | A map from UUIDs to player names.                                                                                                  |
| `sendPluginMessage` | Send a player a [plugin message](https://wiki.vg/Plugin_channels). Arguments: the player (name or UUID), the channel, the message. |
| `sendMessage`       | Send a player a message. Arguments: the player (name or UUID), the message.                                                        |
| `broadcast`         | Broadcast a message to all online players. Arguments: the message.                                                                 |
| `disconnect`        | Disconnect a player from the server. Arguments: the player (name or UUID), the reason (optional)                                   |

## The `registry` table

The `registry` table is used to register commands. It is only available from the `registerCommands` event handler.

| Field        | Description                                       |
|--------------|---------------------------------------------------|
| `addCommand` | Add a command. Arguments: the name of the command |

## The `logger` table

The `logger` table is used to log information the the console. It has the following functions for different logging levels: `trace`, `debug`, `info`, `error`, `warn`. A logger should be initialized in the `init` event handler.

## Chat components

Wherever a chat component is expected (chat messages, disconnect reasons), the plugin can either provide a string or a chat component. Lua tables are a very good approximation for JSON, and as such translating between JSON chat components and tables is not very difficult. See [the wiki.vg documentation for chat components](https://wiki.vg/Chat) for more information.
