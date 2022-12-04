use std::{path::{PathBuf, Path}, str::FromStr};

use mlua::{Function, Table, Lua};

pub struct EventHandlers<'lua> {
    pub init: Option<Function<'lua>>,
    pub player_join: Option<Function<'lua>>,
    pub player_leave: Option<Function<'lua>>,
    pub chat_message: Option<Function<'lua>>,
}   

pub struct Plugin<'lua> {
    pub id: String,
    pub name: String,
    pub version: String,
    pub event_handlers: EventHandlers<'lua>,
}

impl <'lua> Plugin<'lua> {
    pub fn load(path: &Path, lua: &'lua Lua) -> Result<Self, Box<dyn std::error::Error>> {
        let chunk = lua.load(path);
        let module: Table = chunk.eval()?;

        let id: String = module.get("id")?;
        let name: String = module.get("name").unwrap_or_else(|_| id.clone());
        let version: String = module.get("version").unwrap_or_else(|_| "?".to_owned());

        let init: Option<Function<'lua>> = module.get("init").ok();
        let player_join: Option<Function<'lua>> = module.get("playerJoin").ok();
        let player_leave: Option<Function<'lua>> = module.get("playerLeave").ok();
        let chat_message: Option<Function<'lua>> = module.get("chatMessage").ok();

        let event_handlers = EventHandlers { init, player_join, player_leave, chat_message };
        Ok(Plugin { id, name, version, event_handlers })
    }
}
