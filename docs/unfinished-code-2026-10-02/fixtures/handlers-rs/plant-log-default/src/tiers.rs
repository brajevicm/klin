use crate::config;

pub fn load_tiers(path: &str) -> Vec<String> {
    match config::read(path) {
        Ok(config) => config.tiers,
        Err(error) => {
            eprintln!("could not read the config: {error}");
            Vec::new()
        }
    }
}
