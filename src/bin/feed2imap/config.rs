use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::{Context, Error};
use feed2imap::sync::Input;
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
    pub name: String,
    pub email: String,
    pub default_folder: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Feed {
    pub url: String,
}

impl Input for Feed {
    fn url(&self) -> &str {
        &self.url
    }
}

pub fn load<P: AsRef<Path> + Display>(path: P) -> Result<Config, Error> {
    let mut file = File::open(&path).with_context(|| format!("failed opening {}", &path))?;
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

pub fn save<P: AsRef<Path>>(config: &Config, path: P) -> Result<(), Error> {
    let str = toml::to_string_pretty(config)?;
    let mut file = File::create(path)?;
    file.write_all(str.as_bytes())?;
    Ok(())
}
