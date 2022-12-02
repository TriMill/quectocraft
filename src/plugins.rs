use std::{path::{Path, PathBuf}, str::FromStr};

use mlua::{Function, Lua, Error, Table};
use uuid::Uuid;

use crate::network::Player;

pub struct EventHandlers<'lua> {
    init: Option<Function<'lua>>,
    player_join: Option<Function<'lua>>,
    player_leave: Option<Function<'lua>>,
    chat_message: Option<Function<'lua>>,
}   

pub struct Plugin<'lua> {
    pub id: String,
    pub name: String,
    pub version: String,
    pub event_handlers: EventHandlers<'lua>,
}

impl <'lua> Plugin<'lua> {
    pub fn load(path: &str, lua: &'lua Lua) -> Result<Self, Box<dyn std::error::Error>> {
        let path = PathBuf::from_str(path).unwrap();
        let chunk = lua.load(&path);
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

pub struct Plugins<'lua> {
    lua: &'lua Lua,
    plugins: Vec<Plugin<'lua>>
}

impl <'lua> Plugins<'lua> {
    pub fn new(lua: &'lua Lua) -> Result<Self, mlua::Error> {
        let server = lua.create_table()?;
        let players = lua.create_table()?;
        server.set("players", players)?;
        let fn_send = lua.create_function(|_, (uuid, message): (String, String)| {
            Ok(())
        })?;
        server.set("send", fn_send)?;
        lua.globals().set("server", server)?;
        Ok(Self { 
            lua, 
            plugins: Vec::new(),
        })
    }

    pub fn add_plugin(&mut self, pl: Plugin<'lua>) {
        self.plugins.push(pl);
    }

    pub fn init(&self) {
        for pl in &self.plugins {
            if let Some(init) = &pl.event_handlers.init {
                if let Err(e) = init.call::<_, ()>(()) {
                    println!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }

    pub fn player_join(&self, player: &Player) {
        if let Err(e) = self.add_player(player) {
            println!("Error adding player: {}", e);
            return
        }
        for pl in &self.plugins {
            if let Some(init) = &pl.event_handlers.player_join {
                if let Err(e) = init.call::<_, ()>((player.name.as_str(), player.uuid.to_string())) {
                    println!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }

    fn add_player(&self, player: &Player) -> Result<(), mlua::Error> {
        let server: Table = self.lua.globals().get("server")?;
        let players: Table = server.get("players")?;
        players.set(player.uuid.to_string(), player.name.as_str())?;
        Ok(())
    }

    pub fn player_leave(&self, player: &Player) {
        if let Err(e) = self.remove_player(player.uuid) {
            println!("Error removing player: {}", e);
            return
        }
        for pl in &self.plugins {
            if let Some(func) = &pl.event_handlers.player_leave {
                if let Err(e) = func.call::<_, ()>((player.name.as_str(), player.uuid.to_string())) {
                    println!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }

    fn remove_player(&self, uuid: Uuid) -> Result<(), mlua::Error> {
        let server: Table = self.lua.globals().get("server")?;
        let players: Table = server.get("players")?;
        players.set(uuid.to_string(), mlua::Nil)?;
        Ok(())
    }
    
    pub fn chat_message(&self, player: &Player, message: &str) {
        for pl in &self.plugins {
            if let Some(func) = &pl.event_handlers.chat_message {
                if let Err(e) = func.call::<_, ()>((message, player.name.as_str(), player.uuid.to_string())) {
                    println!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }
}
