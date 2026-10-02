use slug::slugify;

#[test]
fn caps_the_length() {
    assert_eq!(slugify(&"x".repeat(100)).unwrap().len(), 40);
}

#[test]
fn rejects_an_empty_title() {
    assert!(slugify("   ").is_ok());
}

#[test]
fn unwrapping_an_empty_title_panics() {
    let _ = slugify("").unwrap();
}
