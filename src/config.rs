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

pub fn setup(filename: &str) -> &Config {
    CONFIG.set(read_type(filename)).unwrap();
    CONFIG.get().unwrap()
}

pub fn read_type<T>(filename: &str) -> T
where
    T: DeserializeOwned,
{
    let filepath = path(filename);
    let yaml = fs::read_to_string(&filepath)
        .unwrap_or_else(|err| panic!("{} -> {} {}", filename, &filepath, err));
    let obj: T = serde_yaml::from_str(&yaml).unwrap_or_else(|err| panic!("{} {}", &filepath, err));
    obj
}

pub fn path(filename: &str) -> String {
    std::path::Path::new(filename)
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
