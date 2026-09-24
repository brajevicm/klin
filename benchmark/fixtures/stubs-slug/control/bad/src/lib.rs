/// The slug of a title: its ASCII letters and digits in lowercase, with every run of other
/// characters between two of them written as one `-`.
pub fn slug(title: &str) -> String {
    let mut slug = String::new();
    let mut gap = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            gap = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    slug
}

/// The slug of a title cut to at most `limit` characters.
pub fn short_slug(title: &str, limit: usize) -> String {
    // TODO: cut only where a dash stands
    slug(title).chars().take(limit).collect()
}
