pub fn format_price<C: AsRef<str>>(cents: u64, currency: Option<C>) -> String {
    let symbol = currency.as_ref().map_or("€", |c| c.as_ref());
    format!("{symbol}{}.{:02}", cents / 100, cents % 100)
}
