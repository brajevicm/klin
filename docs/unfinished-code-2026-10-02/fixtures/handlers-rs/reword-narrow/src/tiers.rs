use std::io::ErrorKind;

use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    match config::read(path) {
        Ok(config) => config.tiers,
        Err(error) if error.kind() != ErrorKind::Interrupted => Vec::new(),
        Err(error) => panic!("{error}"),
    }
}
