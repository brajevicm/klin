use slug::slugify;

pub fn slug_is(title: &str, slug: &str) {
    assert_eq!(slugify(title).unwrap(), slug);
}
