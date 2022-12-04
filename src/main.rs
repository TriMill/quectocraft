use std::time::{Duration, SystemTime};

use env_logger::Env;
use log::info;
use mlua::Lua;
use network::NetworkServer;
use plugins::Plugins;

mod plugins;
mod protocol;
mod network;

pub const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("quectocraft version {}", VERSION);

    let lua = Lua::new();
    let mut plugins = Plugins::new(&lua).expect("Error initializing lua environment");
    std::fs::create_dir_all("plugins").expect("couldn't create the plugins directory");
    plugins.load_plugins();

    info!("{} plugins loaded", plugins.count());
    
    let mut server = NetworkServer::new("127.0.0.1:25565".to_owned(), plugins);
    let sleep_dur = Duration::from_millis(5);
    let mut i = 0;
    loop {
        server.get_new_clients();
        server.handle_connections();
        if i % 1024 == 0 {
            server.send_keep_alive();
            i = 0;
        }
        i += 1;
        std::thread::sleep(sleep_dur);
    }
}
