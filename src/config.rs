use std::{net::IpAddr, fs::OpenOptions};

use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all="lowercase")]
pub enum LoginMode {
    Offline,
    Velocity,
    // TODO online, bungeecord
}

#[derive(Deserialize)]
pub struct Config {
    pub addr: IpAddr,
    pub port: u16,
    pub login: LoginMode,
    pub velocity_secret: Option<String>,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config: Config = serde_json::from_reader(OpenOptions::new().read(true).open("./config.json")?)?;
    if config.login == LoginMode::Velocity && config.velocity_secret.is_none() {
        Err("Velocity is enabled but no secret is configured")?
    }
    Ok(config)
}
