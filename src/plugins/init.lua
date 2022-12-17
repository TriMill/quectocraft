server = { players = {} }
_qc = { responses = {} }

local function to_chat(message, default)
    if message == nil then
        if default ~= nil then
            return default
        else
            error("message must be a string or table")
        end
    elseif type(message) == "table" then
        return message
    elseif type(message) == "string" then
        return { text = message }
    elseif default == nil then
        error("message must be a string or table")
    else
        error("message must be a string, table, or nil for the default message")
    end
end

function server.sendMessage(player, message)
    if type(player) ~= "string" then
        error("player must be a string")
    end
    local message = assert(to_chat(message))
    table.insert(_qc.responses, {type = "message", player = player, message = message})
end

function server.broadcast(message)
    local message = assert(to_chat(message))
    table.insert(_qc.responses, { type = "broadcast", message = message })
end

function server.disconnect(player, reason)
    local reason = assert(to_chat(reason, { translate = "multiplayer.disconnect.generic" }))
    table.insert(_qc.responses, { type = "disconnect", player = player, reason = reason })
end
