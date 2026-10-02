pub fn render_total(cents: i64, currency: &str) -> String {
    if cents == 0 { return "free".to_string(); }
    let negative = cents < 0;
    let absolute = cents.unsigned_abs();
    let whole = absolute / 100;
    let fraction = absolute % 100;
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let symbol = match currency {
        "eur" => "€".to_string(),
        "usd" => "$".to_string(),
        other => format!("{} ", other.to_uppercase()),
    };
    if negative {
        return format!("({symbol}{grouped}.{fraction:02})");
    }
    format!("{symbol}{grouped}.{fraction:02}")
}
