use std::collections::HashMap;

use crate::config;

pub fn load_tiers(path: &str) -> Result<Vec<String>, std::io::Error> {
    let config = config::read(path)?;
    let ranks: HashMap<&str, usize> = HashMap::from([("free", 0), ("pro", 1)]);
    let mut tiers = config.tiers;
    tiers.sort_by_key(|tier| ranks.get(tier.as_str()).copied().unwrap_or(0));
    Ok(tiers)
}
