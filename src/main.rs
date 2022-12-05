use std::borrow::Cow;
use std::time::Duration;
use std::io::Write;

use chrono::Utc;
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
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info")
    ).format(|buf, record| {

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let mut target = Cow::Borrowed(record.target());
        if target.starts_with("quectocraft") {
            target = Cow::Owned(record.target().replacen("quectocraft", "qc", 1));
        }

        let color = match record.level() {
            log::Level::Error => "\x1b[31m",
            log::Level::Warn => "\x1b[33m",
            log::Level::Info => "\x1b[32m",
            log::Level::Debug => "\x1b[37m",
            log::Level::Trace => "\x1b[37m",
        };

        writeln!(buf, "\x1b[90m[\x1b[37m{} {color}{}\x1b[37m {}\x1b[90m]\x1b[0m {}", now, record.level(), target, record.args())
    }).init();


    info!("Starting Quectocraft version {}", VERSION);

    let lua = Lua::new();
    let mut plugins = Plugins::new(&lua).expect("Error initializing lua environment");
    std::fs::create_dir_all("plugins").expect("Couldn't create the plugins directory");
    plugins.load_plugins();
    
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
