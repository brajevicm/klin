pub struct Config {
    pub tiers: Vec<String>,
}

pub fn read(path: &str) -> Result<Config, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    Ok(Config { tiers: text.lines().map(str::to_string).collect() })
}

pub fn save(path: &str, config: &Config) -> Result<(), std::io::Error> {
    std::fs::write(path, config.tiers.join("\n"))
}
