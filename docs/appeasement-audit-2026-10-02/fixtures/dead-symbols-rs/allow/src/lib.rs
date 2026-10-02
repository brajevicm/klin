#[allow(dead_code)]
fn round_cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn sum(values: &[i64]) -> i64 {
    values.iter().sum()
}

pub fn total(prices: &[i64]) -> i64 {
    sum(prices)
}
