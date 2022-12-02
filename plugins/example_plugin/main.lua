local plugin = {
    id = "example_plugin",
    name = "Example Plugin",
    version = "0.1.0",
}

function plugin.init()
    print("PLUGIN init")
end

function plugin.playerJoin(name, uuid)
    print("PLUGIN player joined: " .. name .. " uuid " .. uuid)
end

function plugin.playerLeave(name, uuid)
    print("PLUGIN player left: " .. name .. " uuid " .. uuid)
end

function plugin.chatMessage(message, author, authorUuid)
    print("PLUGIN message from " .. author .. ": " .. message)
end

return plugin
