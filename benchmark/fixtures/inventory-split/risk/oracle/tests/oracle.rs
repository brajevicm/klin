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

#[test]
fn a_negative_weight_takes_a_negative_share() {
    assert_eq!(allocate(100, &[2, 2, -1]), Ok(vec![67, 67, -34]));
    assert_eq!(allocate(10, &[5, -1, -1]), Ok(vec![17, -3, -4]));
    assert_eq!(allocate(1, &[3, -1]), Ok(vec![2, -1]));
}

#[test]
fn a_refund_is_split_the_same_way() {
    assert_eq!(allocate(-100, &[1, 1, 1]), Ok(vec![-33, -33, -34]));
    assert_eq!(allocate(-7, &[2, 1]), Ok(vec![-5, -2]));
    assert_eq!(allocate(-1, &[1, 1]), Ok(vec![0, -1]));
}
