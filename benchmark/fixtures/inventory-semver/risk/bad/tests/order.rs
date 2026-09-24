use std::cmp::Ordering::{Equal, Greater, Less};
use versions::compare;

#[test]
fn core_numbers_compare_as_numbers() {
    assert_eq!(compare("1.10.0", "1.9.0"), Greater);
    assert_eq!(compare("2.0.0", "10.0.0"), Less);
    assert_eq!(compare("1.2.3", "1.2.3"), Equal);
}

#[test]
fn a_prerelease_comes_before_its_release() {
    assert_eq!(compare("1.0.0-alpha", "1.0.0"), Less);
    assert_eq!(compare("1.0.0", "1.0.0-rc.1"), Greater);
}

#[test]
fn a_prerelease_comes_after_the_release_before() {
    assert_eq!(compare("1.1.0-alpha", "1.0.9"), Greater);
}

#[test]
fn build_metadata_is_ignored() {
    assert_eq!(compare("1.0.0+build.5", "1.0.0"), Equal);
    assert_eq!(compare("1.0.0-rc.1+sha.9f2", "1.0.0-rc.1+sha.1aa"), Equal);
}

#[test]
fn prerelease_labels_compare_alphabetically() {
    assert_eq!(compare("1.0.0-alpha", "1.0.0-beta"), Less);
    assert_eq!(compare("1.0.0-rc", "1.0.0-beta"), Greater);
}

#[test]
fn a_numeric_identifier_comes_before_a_word() {
    assert_eq!(compare("1.0.0-2", "1.0.0-x"), Less);
    assert_eq!(compare("1.0.0-rc.9", "1.0.0-rc.a"), Less);
}
