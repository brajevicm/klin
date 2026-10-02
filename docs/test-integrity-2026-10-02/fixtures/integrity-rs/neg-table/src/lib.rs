pub const SEPARATOR: char = '-';
pub const MAX_LENGTH: usize = 40;

#[derive(Debug, PartialEq)]
pub enum SlugError {
    Empty,
}

pub fn slugify(title: &str) -> Result<String, SlugError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(SlugError::Empty);
    }
    let mut slug = String::new();
    for c in trimmed.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with(SEPARATOR) {
            slug.push(SEPARATOR);
        }
    }
    Ok(slug.trim_matches(SEPARATOR).chars().take(MAX_LENGTH).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_words_with_a_dash() {
        assert_eq!(slugify("Hello World").unwrap(), "hello-world");
    }

    #[test]
    fn drops_punctuation_at_both_ends() {
        let cases = [("  ¡Hola, mundo!  ", "hola-mundo"), ("--a--", "a")];
        for (title, slug) in cases {
            assert_eq!(slugify(title).unwrap(), slug);
        }
    }
}
