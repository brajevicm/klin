pub fn base(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>().map_err(|e| e.to_string())
}

pub fn parcel(parts: &[&str]) -> Result<u32, String> {
    let values: Result<Vec<u32>, String> = (0..6)
        .map(|i| parts.get(i).ok_or_else(|| format!("missing field {i}")).and_then(|raw| base(raw)))
        .collect();
    values.map(|v| v[0] * v[1] * v[2] + v[3] * v[4] + v[5])
}
