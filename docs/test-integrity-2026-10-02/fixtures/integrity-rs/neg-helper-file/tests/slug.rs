mod common;

use common::assert_slug_len;
use slug::{slugify, SlugError};

#[test]
fn caps_the_length() {
    assert_slug_len(&"x".repeat(100), 40);
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
