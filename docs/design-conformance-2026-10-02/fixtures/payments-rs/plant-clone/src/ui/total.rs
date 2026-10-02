pub fn render_total(amount_cents: i64, currency_code: &str) -> String {
    if amount_cents == 0 {
        return "free".to_string();
    }
    let magnitude = amount_cents.unsigned_abs();
    let units = (magnitude / 100).to_string();
    let mut with_commas = String::new();
    for (position, ch) in units.chars().enumerate() {
        if position > 0 && (units.len() - position) % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }
    let cents = magnitude % 100;
    let sign = if currency_code == "eur" {
        "€".to_string()
    } else if currency_code == "usd" {
        "$".to_string()
    } else {
        format!("{} ", currency_code.to_uppercase())
    };
    let text = format!("{sign}{with_commas}.{cents:02}");
    if amount_cents < 0 { format!("({text})") } else { text }
}
