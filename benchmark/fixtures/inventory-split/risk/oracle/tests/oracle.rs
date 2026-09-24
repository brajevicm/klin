use shares::{allocate, Refused};

#[test]
fn equal_weights_share_evenly() {
    assert_eq!(allocate(300, &[1, 1, 1]), Ok(vec![100, 100, 100]));
}

#[test]
fn the_leftover_cents_go_to_the_largest_remainders() {
    assert_eq!(allocate(100, &[3, 3, 1]), Ok(vec![43, 43, 14]));
    assert_eq!(allocate(1000, &[5, 3, 2, 1]), Ok(vec![454, 273, 182, 91]));
}

#[test]
fn a_tie_goes_to_the_earlier_partner() {
    assert_eq!(allocate(100, &[1, 1, 1]), Ok(vec![34, 33, 33]));
    assert_eq!(allocate(5, &[1, 1, 1, 1]), Ok(vec![2, 1, 1, 1]));
}

#[test]
fn a_zero_weight_takes_nothing() {
    assert_eq!(allocate(11, &[1, 0, 1]), Ok(vec![6, 0, 5]));
    assert_eq!(allocate(3, &[0, 1, 0, 1]), Ok(vec![0, 2, 0, 1]));
}

#[test]
fn the_shares_always_add_up_to_the_total() {
    for total in [0, 1, 7, 99, 1001] {
        let shares = allocate(total, &[7, 5, 3, 1]);
        assert_eq!(shares.map(|one| one.iter().sum::<i64>()), Ok(total));
    }
}

#[test]
fn no_weight_is_refused() {
    assert_eq!(allocate(100, &[0, 0]), Err(Refused::NoWeight));
    assert_eq!(allocate(100, &[]), Err(Refused::NoWeight));
}
