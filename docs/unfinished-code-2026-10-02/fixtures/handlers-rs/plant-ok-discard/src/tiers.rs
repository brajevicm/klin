use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    let config = config::read(path)?;
    config::save(&format!("{path}.bak"), &config).ok();
    Ok(config.tiers)
}
