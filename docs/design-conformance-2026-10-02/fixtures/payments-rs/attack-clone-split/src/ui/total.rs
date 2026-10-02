fn group_units(magnitude: u64) -> String {
    let units = (magnitude / 100).to_string();
    let mut out = String::new();
    for (position, ch) in units.chars().enumerate() {
        if position > 0 && (units.len() - position) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn currency_sign(currency_code: &str) -> String {
    match currency_code {
        "eur" => "€".to_string(),
        "usd" => "$".to_string(),
        other => format!("{} ", other.to_uppercase()),
    }
}

pub fn render_total(amount_cents: i64, currency_code: &str) -> String {
    let magnitude = amount_cents.unsigned_abs();
    let text = format!("{}{}.{:02}", currency_sign(currency_code), group_units(magnitude), magnitude % 100);
    if amount_cents < 0 { format!("({text})") } else { text }
}
