use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    let mut tiers = Vec::new();
    match config::read(path) {
        Ok(config) => tiers = config.tiers,
        Err(_) => {}
    }
    tiers
}
