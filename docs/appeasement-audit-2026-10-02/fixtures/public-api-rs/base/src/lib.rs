pub fn format_price(cents: u64) -> String {
    format!("€{}.{:02}", cents / 100, cents % 100)
}
