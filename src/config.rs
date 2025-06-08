use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;

pub static FILENAME: &'static str = "config.yaml";
pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub geth_url: String,
}

pub fn read_type<T>(filename: &str) -> T
where
    T: DeserializeOwned,
{
    let yaml = fs::read_to_string(filename).unwrap_or_else(|err| panic!("{} {}", filename, err));
    let obj: T = serde_yaml::from_str(&yaml).unwrap_or_else(|err| panic!("{} {}", filename, err));
    obj
}
