use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
  pub port: u64,
  pub boot_node: bool
}

pub fn get_config() -> Config {
  let contents = fs::read_to_string("config.json")
    .expect("Couldn't read config.json");
  let cfg: Config = serde_json::from_str(&contents).expect("Config file not in a correct format");
  cfg
}