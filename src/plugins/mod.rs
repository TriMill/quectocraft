use std::{fs::read_dir, rc::Rc, cell::RefCell, collections::HashMap};

use log::{warn, info};
use mlua::{Lua, Table, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{network::Player, protocol::command::Commands};

use self::plugin::Plugin;

mod init_lua;
mod plugin;

#[derive(Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Response {
    #[serde(rename = "message")]
    Message { player: String, message: serde_json::Value },
    #[serde(rename = "plugin_message")]
    PluginMessage { player: String, channel: String, data: Vec<u8> },
    #[serde(rename = "broadcast")]
    Broadcast { message: serde_json::Value },
    #[serde(rename = "disconnect")]
    Disconnect { player: String, reason: serde_json::Value },
}


pub struct Plugins<'lua> {
    lua: &'lua Lua,
    plugins: Vec<Plugin<'lua>>,
    cmd_owners: HashMap<String, usize>,
}

impl <'lua> Plugins<'lua> {
    pub fn new(lua: &'lua Lua) -> Result<Self, mlua::Error> {
        init_lua::init(lua)?;
        Ok(Self { 
            lua, 
            plugins: Vec::new(),
            cmd_owners: HashMap::new(),
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
            let pl = Plugin::load(&path, self.lua).expect("error loading plugin");
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

    pub fn register_commands(&mut self, commands: Commands) -> Result<Commands, mlua::Error> {
        let commands = Rc::new(RefCell::new(commands));
        let cmd_owners = Rc::new(RefCell::new(HashMap::new()));
        for (i, pl) in self.plugins.iter().enumerate() {
            let commands_2 = commands.clone();
            let cmd_owners_2 = cmd_owners.clone();
            let pl_id = pl.id.clone();
            let add_command = self.lua.create_function(move |_, name: String| {
                let scoped_name = format!("{}:{}", pl_id, name);
                let mut cmds = commands_2.borrow_mut();
                let id1 = cmds.create_simple_cmd(&name);
                let id2 = cmds.create_simple_cmd(&scoped_name);
                if id1.is_none() || id2.is_none() { 
                    return Ok(mlua::Nil)
                }
                cmd_owners_2.borrow_mut().insert(name, i);
                cmd_owners_2.borrow_mut().insert(scoped_name, i);
                Ok(mlua::Nil)
            })?;
            let registry = self.lua.create_table()?;
            registry.set("addCommand", add_command)?;
            if let Some(init) = &pl.event_handlers.register_commands {
                if let Err(e) = init.call::<_, ()>((registry.clone(),)) {
                    warn!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
        let cb = commands.borrow();
        self.cmd_owners = (*cmd_owners.borrow()).clone();
        Ok((*cb).clone())
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

    pub fn command(&self, player: &Player, command: &str, args: &str) {
        if let Some(owner) = self.cmd_owners.get(command) {
            let pl = &self.plugins[*owner];
            if let Some(func) = &pl.event_handlers.command {
                if let Err(e) = func.call::<_, ()>((command, args, player.name.as_str(), player.uuid.to_string())) {
                    warn!("Error in plugin {}: {}", pl.name, e);
                }
            } else {
                warn!("Plugin {} registered a command but no command handler was found", pl.id);
            }
        }
    }

    pub fn plugin_message(&self, player: &Player, channel: &str, data: &[u8]) {
        for pl in &self.plugins {
            if let Some(func) = &pl.event_handlers.plugin_message {
                if let Err(e) = func.call::<_, ()>((channel, data, player.name.as_str(), player.uuid.to_string())) {
                    warn!("Error in plugin {}: {}", pl.name, e);
                }
            }
        }
    }
}
