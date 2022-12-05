use log::{info, warn, trace, error, debug};
use mlua::{Lua, chunk};


pub fn init<'lua>(lua: &'lua Lua) -> Result<(), mlua::Error> {
    macro_rules! log_any {
        ($level:tt) => {
            lua.create_function(|_, args: (String, String)| {
                $level!(target: &args.0, "{}", args.1);
                Ok(())
            })
        }
    }
    let log_trace = log_any!(trace)?;
    let log_debug = log_any!(debug)?;
    let log_info = log_any!(info)?;
    let log_warn = log_any!(warn)?;
    let log_error = log_any!(error)?;
    lua.load(chunk!{
        server = { players = {} }
        _qc = { responses = {} }
        function server.initLogger(plugin)
            local id = "pl::" .. assert(plugin["id"])
            return {
                trace = function(msg) $log_trace(id, msg) end,
                debug = function(msg) $log_debug(id, msg) end,
                info = function(msg) $log_info(id, msg) end,
                warn = function(msg) $log_warn(id, msg) end,
                error = function(msg) $log_error(id, msg) end,
            }
        end
        
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
    }).exec()?;
    Ok(())
}
