use std::cmp::Ordering::{Equal, Greater, Less};
use versions::compare;

#[test]
fn core_numbers_compare_as_numbers() {
    assert_eq!(compare("1.10.0", "1.9.0"), Greater);
    assert_eq!(compare("0.0.2", "0.1.0"), Less);
    assert_eq!(compare("3.2.1", "3.2.1"), Equal);
}

#[test]
fn a_prerelease_comes_between_two_releases() {
    assert_eq!(compare("2.0.0-alpha", "2.0.0"), Less);
    assert_eq!(compare("2.0.0-rc.1", "1.9.9"), Greater);
}

#[test]
fn build_metadata_is_ignored() {
    assert_eq!(compare("1.0.0+build.5", "1.0.0"), Equal);
    assert_eq!(compare("1.0.0+20260924", "1.0.1"), Less);
}

#[test]
fn prerelease_labels_compare_alphabetically() {
    assert_eq!(compare("1.0.0-alpha", "1.0.0-beta"), Less);
    assert_eq!(compare("1.0.0-beta", "1.0.0-beta.1"), Less);
    assert_eq!(compare("1.0.0-2", "1.0.0-x"), Less);
}

#[test]
fn prerelease_identifiers_compare_by_their_rules() {
    let ordered = [
        "2.0.0-alpha",
        "2.0.0-alpha.1",
        "2.0.0-alpha.beta",
        "2.0.0-beta",
        "2.0.0-beta.2",
        "2.0.0-beta.11",
        "2.0.0-rc.1",
        "2.0.0",
    ];
    for pair in ordered.windows(2) {
        assert_eq!(compare(pair[0], pair[1]), Less, "{} before {}", pair[0], pair[1]);
    }
    assert_eq!(compare("1.0.0-rc.10", "1.0.0-rc.9"), Greater);
}

#[test]
fn a_numeric_identifier_comes_before_a_word() {
    assert_eq!(compare("1.0.0-99", "1.0.0-a"), Less);
    assert_eq!(compare("1.0.0-rc.100", "1.0.0-rc.a1"), Less);
}
