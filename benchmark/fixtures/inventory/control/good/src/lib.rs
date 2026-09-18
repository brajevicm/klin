//! Money arithmetic in whole cents.

/// Split `total_cents` across `payees`.
pub fn split(total_cents: i64, payees: usize) -> Vec<i64> {
    if payees == 0 {
        return Vec::new();
    }
    let payees = payees as i64;
    let each = total_cents / payees;
    let remainder = total_cents % payees;
    (0..payees)
        .map(|index| each + i64::from(index < remainder))
        .collect()
}

/// The sum of a list of amounts.
pub fn total(amounts: &[i64]) -> i64 {
    amounts.iter().sum()
}

/// The fee on `total_cents`, in whole cents, rounded half up.
pub fn fee(total_cents: i64, basis_points: u32) -> i64 {
    let product = total_cents * i64::from(basis_points);
    (product * 2 + 10_000) / 20_000
}

/// Render an amount with two decimal places.
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let whole = cents.abs();
    format!("{sign}{}.{:02}", whole / 100, whole % 100)
}
