use std::{net::IpAddr, fs::OpenOptions};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub addr: IpAddr,
    pub port: u16,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config = serde_json::from_reader(OpenOptions::new().read(true).open("./config.json")?)?;
    Ok(config)
}
