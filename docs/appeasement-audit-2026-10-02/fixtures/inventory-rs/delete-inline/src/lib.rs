pub fn double(value: i64) -> i64 {
    value * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_a_positive() {
        assert_eq!(double(2), 4);
    }
}
