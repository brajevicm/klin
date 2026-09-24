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
    let mut shares: Vec<i64> = weights.iter().map(|weight| (total * weight).div_euclid(sum)).collect();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&at| (std::cmp::Reverse((total * weights[at]).rem_euclid(sum)), at));
    let leftover = total - shares.iter().sum::<i64>();
    for &at in order.iter().take(leftover as usize) {
        shares[at] += 1;
    }
    Ok(shares)
}

/// Splits `total` cents between `partners` partners who all hold the same weight.
pub fn split_evenly(total: i64, partners: usize) -> Result<Vec<i64>, Refused> {
    allocate(total, &vec![1; partners])
}
