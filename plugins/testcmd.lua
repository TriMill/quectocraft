local plugin = {
    id = "testcmd",
    name = "TestCmd",
    description = "eufdahjklfhjakl",
    authors = { "trimill" },
    version = "0.1.0",
}

local logger = nil

function plugin.init()
    logger = server.initLogger(plugin)
end

function plugin.registerCommands(registry)
    registry.addCommand("test")
end

function plugin.command(command, args, name, uuid)
    logger.info("player " .. name .. " ran /" .. command .. " " .. args)
end

function plugin.playerJoin(name, uuid)
    logger.info("player joined: " .. name .. " with uuid " .. uuid)
end

return plugin
