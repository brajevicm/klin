use std::io::ErrorKind;

use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    match config::read(path) {
        Ok(config) => config.tiers,
        Err(error) if error.kind() == ErrorKind::NotFound => vec!["free".to_string()],
        Err(error) => panic!("cannot read {path}: {error}"),
    }
}
