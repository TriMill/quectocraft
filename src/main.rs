use std::time::Duration;

use mlua::Lua;
use network::NetworkServer;
use plugins::{Plugin, Plugins};

mod plugins;
mod protocol;
mod network;

fn main() {
    let lua = Lua::new();
    let plugin = Plugin::load("plugins/example_plugin/main.lua".into(), &lua).unwrap();
    let mut plugins = Plugins::new(&lua).unwrap();
    plugins.add_plugin(plugin);
    
    let mut server = NetworkServer::new("127.0.0.1:25565".to_owned(), plugins);
    let sleep_dur = Duration::from_millis(5);
    let mut i = 0;
    loop {
        server.get_new_clients();
        server.handle_connections();
        std::thread::sleep(sleep_dur);
        if i % 1024 == 0 {
            server.send_keep_alive();
            i = 0;
        }
        i += 1;
    }
}
