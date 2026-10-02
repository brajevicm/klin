use crate::config;
use crate::safely::or_empty;

pub fn load_tiers(path: &str) -> Vec<String> {
    or_empty(config::read(path).map(|config| config.tiers))
}
