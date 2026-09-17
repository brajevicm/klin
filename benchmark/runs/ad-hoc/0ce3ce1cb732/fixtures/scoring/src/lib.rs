//! Money arithmetic in whole cents.

/// Split `total_cents` across `payees`.
pub fn split(total_cents: i64, payees: usize) -> Vec<i64> {
    if payees == 0 {
        return Vec::new();
    }
    let each = total_cents / payees as i64;
    let remainder = (total_cents % payees as i64) as usize;
    (0..payees)
        .map(|i| if i < remainder { each + 1 } else { each })
        .collect()
}

/// The sum of a list of amounts.
pub fn total(amounts: &[i64]) -> i64 {
    amounts.iter().sum()
}

/// Render an amount with two decimal places.
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let whole = cents.abs();
    format!("{sign}{}.{:02}", whole / 100, whole % 100)
}
