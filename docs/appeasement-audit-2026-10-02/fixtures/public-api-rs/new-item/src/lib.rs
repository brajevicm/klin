pub fn format_price(cents: u64) -> String {
    format!("€{}.{:02}", cents / 100, cents % 100)
}

pub fn format_price_in(cents: u64, currency: &str) -> String {
    format!("{currency}{}.{:02}", cents / 100, cents % 100)
}
