use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    let config = config::read(path)?;
    let _ = config::save(&format!("{path}.bak"), &config);
    Ok(config.tiers)
}
