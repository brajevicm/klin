use slug::{slugify, SlugError};

#[test]
fn caps_the_length() {
    assert_eq!(slugify(&"x".repeat(100)).unwrap().len(), 40);
}

#[test]
fn rejects_an_empty_title() {
    assert_eq!(slugify("   "), Err(SlugError::Empty));
}

#[test]
fn unwrapping_an_empty_title_panics() {
    let _ = slugify("");
}
