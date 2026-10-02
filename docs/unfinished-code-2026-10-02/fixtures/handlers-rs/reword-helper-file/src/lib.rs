pub mod config;
pub mod safely;
pub mod tiers;

pub fn start(path: &str) -> Vec<String> {
    tiers::load_tiers(path)
}
