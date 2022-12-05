use std::fs::read_dir;

use log::{warn, info};
use mlua::{Lua, Table, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::network::Player;

use self::plugin::Plugin;

mod init_lua;
mod plugin;

#[derive(Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Response {
    #[serde(rename = "message")]
    Message { player: String, message: serde_json::Value },
    #[serde(rename = "broadcast")]
    Broadcast { message: serde_json::Value },
    #[serde(rename = "disconnect")]
    Disconnect { player: String, reason: serde_json::Value },
}


pub struct Plugins<'lua> {
    lua: &'lua Lua,
    plugins: Vec<Plugin<'lua>>
}

impl <'lua> Plugins<'lua> {
    pub fn new(lua: &'lua Lua) -> Result<Self, mlua::Error> {
        init_lua::init(lua)?;
        Ok(Self { 
            lua, 
            plugins: Vec::new(),
        })
    }

    pub fn load_plugins(&mut self) {
        let files = read_dir("plugins").expect("couldn't read plugins directory");
        for file in files {
            let file = file.expect("couldn't read contents of plugins directory");
            let path = if file.file_type().expect("couldn't get type of plugin file").is_dir() {
                let mut main = file.path();
                main.push("main.lua");
                main
            } else {
                file.path()
            };
            let pl = Plugin::load(&path, &self.lua).expect("error loading plugin");
            self.plugins.push(pl);
            info!("Loaded plugin '{}'", file.file_name().to_string_lossy());
        }
    }

    pub fn get_responses(&self) -> Vec<Response> {
        match self.get_responses_inner() {
            Ok(x) => x,
            Err(e) => {
                warn!("Error getting responses: {}", e);
                Vec::new()
            }
        }
    }

    fn get_responses_inner(&self) -> Result<Vec<Response>, Box<dyn std::error::Error>> {
        let qc: Table = self.lua.globals().get("_qc")?;
        let responses: Vec<Response> = self.lua.from_value(qc.get("responses")?)?;
        qc.set("responses", self.lua.create_table()?)?;
        Ok(responses)
    }

    pub fn init(&self) {
        for pl in &self.plugins {
            if let Some(init) = &pl.event_handlers.init {
                if let Err(e) = init.call::<_, ()>(()) {
                    warn!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }

    pub fn player_join(&self, player: &Player) {
        if let Err(e) = self.add_player(player) {
            warn!("Error adding player: {}", e);
            return
        }
        for pl in &self.plugins {
            if let Some(init) = &pl.event_handlers.player_join {
                if let Err(e) = init.call::<_, ()>((player.name.as_str(), player.uuid.to_string())) {
                    warn!("Error in plugin {}: {}", pl.name, e);
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
            warn!("Error removing player: {}", e);
            return
        }
        for pl in &self.plugins {
            if let Some(func) = &pl.event_handlers.player_leave {
                if let Err(e) = func.call::<_, ()>((player.name.as_str(), player.uuid.to_string())) {
                    warn!("Error in plugin {}: {}", pl.name, e);
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
                    warn!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }
}
