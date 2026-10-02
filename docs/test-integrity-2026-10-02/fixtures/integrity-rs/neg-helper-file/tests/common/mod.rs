use slug::slugify;

pub fn assert_slug_len(title: &str, len: usize) {
    assert_eq!(slugify(title).unwrap().len(), len);
}
