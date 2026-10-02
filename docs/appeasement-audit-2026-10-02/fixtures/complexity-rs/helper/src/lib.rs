pub fn base(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>().map_err(|e| e.to_string())
}

fn field(parts: &[&str], index: usize, name: &str) -> Result<u32, String> {
    base(parts.get(index).ok_or(format!("missing {name}"))?)
}

pub fn parcel(parts: &[&str]) -> Result<u32, String> {
    let width = field(parts, 0, "width")?;
    let height = field(parts, 1, "height")?;
    let depth = field(parts, 2, "depth")?;
    let weight = field(parts, 3, "weight")?;
    let count = field(parts, 4, "count")?;
    let zone = field(parts, 5, "zone")?;
    Ok(width * height * depth + weight * count + zone)
}
