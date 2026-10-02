pub fn base(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>().map_err(|e| e.to_string())
}

pub fn parcel(parts: &[&str]) -> Result<u32, String> {
    let width = base(parts.get(0).ok_or("missing width")?)?;
    let height = base(parts.get(1).ok_or("missing height")?)?;
    let depth = base(parts.get(2).ok_or("missing depth")?)?;
    let weight = base(parts.get(3).ok_or("missing weight")?)?;
    let count = base(parts.get(4).ok_or("missing count")?)?;
    let zone = base(parts.get(5).ok_or("missing zone")?)?;
    Ok(width * height * depth + weight * count + zone)
}
