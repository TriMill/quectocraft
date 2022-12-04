use std::{path::Path, fs::{self, read_dir}};

use log::warn;
use mlua::{Lua, Table, chunk, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::network::Player;

use self::plugin::Plugin;

mod plugin;

pub struct Plugins<'lua> {
    lua: &'lua Lua,
    plugins: Vec<Plugin<'lua>>
}

impl <'lua> Plugins<'lua> {
    pub fn new(lua: &'lua Lua) -> Result<Self, mlua::Error> {
        lua.load(chunk!{
            server = { players = {} }
            _qc = { responses = {} }
            function server.sendMessage(player, message)
                if type(player) ~= "string" then
                    error("player must be a string")
                end
                if type(message) == "table" then
                    table.insert(_qc.responses, {type = "message", player = player, message = message})
                elseif type(message) == "string" then
                    table.insert(_qc.responses, {type = "message", player = player, message = { text = message}})
                else
                    error("message must be a string or table")                    
                end
            end
            function server.broadcast(message)
                if type(message) == "table" then
                    table.insert(_qc.responses, { type = "broadcast", message = message })
                elseif type(message) == "string" then
                    table.insert(_qc.responses, { type = "broadcast", message = { text = message } })
                else
                    error("message must be a string or table")                    
                end
            end
            function server.initLogger(plugin)
                local function log_for(level)
                    return function(message) 
                        table.insert(_qc.responses, { 
                            type = "log", origin = plugin.id, message = message, level = level
                        }) 
                    end
                end
                return {
                    trace = log_for(0),
                    debug = log_for(1),
                    info = log_for(2),
                    warn = log_for(3),
                    error = log_for(4),
                }
            end
        }).exec()?;
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
        }
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    pub fn get_responses(&self) -> Vec<Response> {
        match self.get_responses_inner() {
            Ok(x) => x,
            Err(e) => {
                warn!("error getting responses: {}", e);
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

#[derive(Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Response {
    #[serde(rename = "log")]
    Log { level: i32, origin: String, message: String },
    #[serde(rename = "message")]
    Message { player: String, message: serde_json::Value },
    #[serde(rename = "broadcast")]
    Broadcast { message: serde_json::Value },
}
