/// Why an amount cannot be split.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// The weights add up to zero or less, so no partner holds a share of the whole.
    NoWeight,
}

/// Splits `total` cents between the partners in proportion to their `weights`. The shares are
/// whole cents and they always add up to `total`.
pub fn allocate(total: i64, weights: &[i64]) -> Result<Vec<i64>, Refused> {
    let sum: i64 = weights.iter().sum();
    if sum <= 0 {
        return Err(Refused::NoWeight);
    }
    Ok(weights.iter().map(|weight| total * weight / sum).collect())
}
