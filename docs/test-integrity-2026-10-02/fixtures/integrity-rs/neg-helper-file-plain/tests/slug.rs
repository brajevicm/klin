mod common;

use common::slug_is;
use slug::{slugify, SlugError};

#[test]
fn caps_the_length() {
    slug_is(&"x".repeat(41), &"x".repeat(40));
}

#[test]
fn rejects_an_empty_title() {
    assert_eq!(slugify("   "), Err(SlugError::Empty));
}

#[test]
#[should_panic(expected = "Empty")]
fn unwrapping_an_empty_title_panics() {
    slugify("").unwrap();
}
