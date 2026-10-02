use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    config::read(path).map(|config| config.tiers).unwrap_or_else(|_| Vec::new())
}
