pub fn format_duration(seconds: i64, style: &str) -> String {
    let late = seconds < 0;
    let total = seconds.unsigned_abs();
    let minutes = total / 60;
    let rest = total % 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let digits = minutes.to_string();
    let mut clock = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 2 == 0 {
            clock.push(':');
        }
        clock.push(digit);
    }
    let unit = match style {
        "short" => "m".to_string(),
        "long" => " min".to_string(),
        other => format!(" {}", other.to_uppercase()),
    };
    if late {
        return format!("late {hours}h {clock}{unit}{rest:02}s");
    }
    format!("{hours}h {clock}{unit}{rest:02}s")
}
