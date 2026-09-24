use shares::{allocate, split_evenly, Refused};

#[test]
fn an_even_split_gives_the_leftover_to_the_earliest_partners() {
    assert_eq!(split_evenly(100, 3), Ok(vec![34, 33, 33]));
    assert_eq!(split_evenly(-100, 3), Ok(vec![-33, -33, -34]));
}

#[test]
fn a_split_between_no_partners_is_refused() {
    assert_eq!(split_evenly(100, 0), Err(Refused::NoWeight));
}

#[test]
fn the_weighted_split_is_unchanged() {
    assert_eq!(allocate(100, &[3, 3, 1]), Ok(vec![43, 43, 14]));
    assert_eq!(allocate(10, &[5, -1, -1]), Ok(vec![17, -3, -4]));
}
