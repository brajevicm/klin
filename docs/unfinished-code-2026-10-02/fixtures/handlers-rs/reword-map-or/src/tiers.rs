use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    config::read(path).map_or(Vec::new(), |config| config.tiers)
}
