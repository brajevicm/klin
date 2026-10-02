use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    if let Ok(config) = config::read(path) {
        return config.tiers;
    }
    Vec::new()
}
