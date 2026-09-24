use shares::{allocate, Refused};

#[test]
fn equal_weights_share_evenly() {
    assert_eq!(allocate(300, &[1, 1, 1]), Ok(vec![100, 100, 100]));
}

#[test]
fn the_shares_add_up_to_the_total() {
    let shares = allocate(100, &[3, 3, 1]);
    assert_eq!(shares.map(|one| one.iter().sum::<i64>()), Ok(100));
}

#[test]
fn the_leftover_cents_go_to_the_largest_remainders() {
    assert_eq!(allocate(100, &[3, 3, 1]), Ok(vec![43, 43, 14]));
    assert_eq!(allocate(10, &[1, 2, 7]), Ok(vec![1, 2, 7]));
}

#[test]
fn a_tie_goes_to_the_earlier_partner() {
    assert_eq!(allocate(100, &[1, 1, 1]), Ok(vec![34, 33, 33]));
}

#[test]
fn a_zero_weight_takes_nothing() {
    assert_eq!(allocate(11, &[1, 0, 1]), Ok(vec![6, 0, 5]));
}

#[test]
fn no_weight_is_refused() {
    assert_eq!(allocate(100, &[0, 0]), Err(Refused::NoWeight));
    assert_eq!(allocate(100, &[]), Err(Refused::NoWeight));
}
