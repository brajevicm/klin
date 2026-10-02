pub fn fee(cents: u64) -> u64 {
    cents / 20
}

#[cfg(any())]
pub fn express_fee(cents: u64) -> u64 {
    fee(cents) + "5"
}
