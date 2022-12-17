use log::{info, warn, trace, error, debug};
use mlua::{Lua, chunk};
use crate::VERSION;


pub fn init(lua: &Lua) -> Result<(), mlua::Error> {
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
    lua.load(include_str!("init.lua")).exec()?;
    lua.load(chunk!{
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

        server.version = $VERSION
    }).exec()?;
        
    Ok(())
}
