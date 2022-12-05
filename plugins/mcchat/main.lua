local plugin = {
    id = "mcchat",
    name = "MCChat",
    description = "Provides Minecraft-style chat. Messages sent by one client will be broadcasted to every client.",
    authors = { "trimill" },
    version = "0.1.0",
}

local logger = nil

function plugin.init()
    logger = server.initLogger(plugin)
    logger.info("MCChat version " .. plugin.version)
end

function plugin.playerJoin(name)
    logger.info(name .. " joined the game")
    server.broadcast({
        translate = "multiplayer.player.joined",
        with = { {text = name} }, 
        color = "yellow"
    })
end

function plugin.playerLeave(name)
    logger.info(name .. " left the game")
    server.broadcast({
        translate = "multiplayer.player.left",
        with = { {text = name} }, 
        color = "yellow"
    })
end

function plugin.chatMessage(message, author)
    logger.info("<" .. author .. "> " .. message)
    server.broadcast({
        translate = "chat.type.text",
        with = {
            {text = author},
            {text = message}
        }
    })
end

return plugin
