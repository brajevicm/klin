use slug::{slugify, SlugError};

#[test]
fn caps_the_length() {
    assert_eq!(slugify(&"x".repeat(100)).unwrap().len(), 40);
}

#[test]
fn rejects_an_empty_title() {
    if let Err(error) = slugify("   ") {
        assert_eq!(error, SlugError::Empty);
    }
}

#[test]
#[should_panic]
fn unwrapping_an_empty_title_panics() {
    slugify("").unwrap();
    panic!("Empty");
}
