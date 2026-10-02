use std::io::ErrorKind;

use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    let config = config::read(path)?;
    if let Err(error) = std::fs::remove_file(format!("{path}.lock")) {
        if error.kind() != ErrorKind::NotFound {
            panic!("{error}");
        }
    }
    Ok(config.tiers)
}
