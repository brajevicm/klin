use std::fmt::Write;

use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    let config = config::read(path)?;
    let mut log = String::new();
    let _ = write!(log, "{} tiers", config.tiers.len());
    Ok(config.tiers)
}
