fn sum(values: &[i64]) -> i64 {
    values.iter().sum()
}

fn round_cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn total(prices: &[i64]) -> i64 {
    sum(prices)
}

pub fn average(prices: &[i64]) -> f64 {
    round_cents(sum(prices) as f64 / prices.len().max(1) as f64)
}
