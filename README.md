# Quectocraft

Quectocraft is a minimal, extensible, efficient Minecraft server implementation written in Rust and Lua.

## Goals

- Minimal: By default, Quectocraft does very little by itself. It accepts connections, encodes and decodes packets, and handles the login sequence for you, but everything else must be done via plugins.
- Extensible: Via its Lua plugin system, Quectocraft can be configured to do a variety of things.
- Efficient: The vanilla Minecraft server, and even more efficient servers like Spigot and Paper, all use significant amounts of CPU even while idling with no players connected. Due to its low CPU and memory usage, Quectocraft is suitable for running on lower-end systems, or alongside another server without causing additional lag.

## Why?

I'm mostly just writing this for fun, but here are some potential applications:
- A lobby for a server network
- A queue that players have to wait in before joining another server
- A server to send players to if they are AFK for too long
