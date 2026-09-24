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

/// The slug of a title cut to at most `limit` characters, only where a `-` stands. A first
/// word longer than `limit` is cut at `limit`.
pub fn short_slug(title: &str, limit: usize) -> String {
    let whole = slug(title);
    if whole.len() <= limit {
        return whole;
    }
    match whole[..=limit].rfind('-') {
        Some(0) | None => whole[..limit].to_string(),
        Some(dash) => whole[..dash].to_string(),
    }
}
