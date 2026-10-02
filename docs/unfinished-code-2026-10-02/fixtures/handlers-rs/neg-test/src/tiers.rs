use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    Ok(config::read(path)?.tiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_empty() {
        let tiers = config::read("none").map(|config| config.tiers).unwrap_or_default();
        assert!(tiers.is_empty());
    }
}
