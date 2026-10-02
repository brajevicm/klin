pub mod config;

pub fn start(path: &str) -> Vec<String> {
    config::read(path).map(|config| config.tiers).unwrap_or_else(|error| panic!("{error}"))
}
