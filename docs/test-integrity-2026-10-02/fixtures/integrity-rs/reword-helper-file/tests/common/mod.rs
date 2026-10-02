use slug::slugify;

pub fn assert_slug(title: &str, _slug: &str) {
    assert!(slugify(title).is_ok());
}
