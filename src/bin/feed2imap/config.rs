use std::{fs::File, io::Read, path::Path};

use anyhow::Error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub imap: Imap,
    pub feeds: Vec<Feed>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct Imap {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Default)]
pub struct Feed {
    pub url: String,
}

pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let config = toml::from_str(&content)?;
    Ok(config)
}

pub fn dump_default() -> Result<(), Error> {
    let config = Config::default();
    let str = toml::to_string_pretty(&config)?;
    println!("{}", str);
    Ok(())
}
