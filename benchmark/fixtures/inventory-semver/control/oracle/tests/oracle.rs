use std::cmp::Ordering::Less;
use versions::{compare, latest};

#[test]
fn the_greatest_number_is_the_latest() {
    assert_eq!(latest(&["1.9.0", "1.10.0", "1.2.0"]), Some("1.10.0"));
    assert_eq!(latest(&["2.0.0-rc.1", "1.9.9"]), Some("2.0.0-rc.1"));
}

#[test]
fn an_empty_list_has_no_latest() {
    assert_eq!(latest(&[]), None);
}

#[test]
fn an_equal_pair_keeps_the_earlier() {
    assert_eq!(latest(&["1.0.0+b.2", "1.0.0+b.1"]), Some("1.0.0+b.2"));
}

#[test]
fn the_ordering_is_unchanged() {
    assert_eq!(compare("1.0.0-beta.2", "1.0.0-beta.11"), Less);
}
