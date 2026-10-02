use crate::config;

pub fn find_tiers(path: &str) -> Option<Vec<String>> {
    match config::read(path) {
        Ok(config) => Some(config.tiers),
        Err(_) => None,
    }
}

pub fn load_tiers(path: &str) -> Vec<String> {
    find_tiers(path).unwrap_or_else(|| vec!["free".to_string()])
}
