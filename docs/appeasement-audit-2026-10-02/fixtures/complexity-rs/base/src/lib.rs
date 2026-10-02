pub fn base(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>().map_err(|e| e.to_string())
}
