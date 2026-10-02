use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    let fallback = Vec::new();
    match config::read(path) {
        Ok(config) => config.tiers,
        Err(_) => fallback,
    }
}
