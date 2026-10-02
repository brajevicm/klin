use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    match config::read(path) {
        Ok(config) => config.tiers,
        Err(_) => Vec::new(),
    }
}
