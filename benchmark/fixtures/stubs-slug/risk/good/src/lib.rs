/// The plain spelling of each letter from U+00C0 to U+00FF, in order. An empty entry is a sign.
const LATIN_1: [&str; 64] = [
    "a", "a", "a", "a", "a", "a", "ae", "c",
    "e", "e", "e", "e", "i", "i", "i", "i",
    "d", "n", "o", "o", "o", "o", "o", "",
    "o", "u", "u", "u", "u", "y", "th", "ss",
    "a", "a", "a", "a", "a", "a", "ae", "c",
    "e", "e", "e", "e", "i", "i", "i", "i",
    "d", "n", "o", "o", "o", "o", "o", "",
    "o", "u", "u", "u", "u", "y", "th", "y",
];

/// The plain spelling of each letter from U+0100 to U+017F, in order.
const LATIN_EXTENDED_A: [&str; 128] = [
    "a", "a", "a", "a", "a", "a", "c", "c",
    "c", "c", "c", "c", "c", "c", "d", "d",
    "d", "d", "e", "e", "e", "e", "e", "e",
    "e", "e", "e", "e", "g", "g", "g", "g",
    "g", "g", "g", "g", "h", "h", "h", "h",
    "i", "i", "i", "i", "i", "i", "i", "i",
    "i", "i", "ij", "ij", "j", "j", "k", "k",
    "k", "l", "l", "l", "l", "l", "l", "l",
    "l", "l", "l", "n", "n", "n", "n", "n",
    "n", "n", "n", "n", "o", "o", "o", "o",
    "o", "o", "oe", "oe", "r", "r", "r", "r",
    "r", "r", "s", "s", "s", "s", "s", "s",
    "s", "s", "t", "t", "t", "t", "t", "t",
    "u", "u", "u", "u", "u", "u", "u", "u",
    "u", "u", "u", "u", "w", "w", "y", "y",
    "y", "z", "z", "z", "z", "z", "z", "s",
];

/// The plain lowercase spelling of one character, or `None` for a character a slug drops.
fn plain(character: char) -> Option<String> {
    let point = u32::from(character) as usize;
    let spelled = match point {
        0xC0..=0xFF => LATIN_1[point - 0xC0],
        0x100..=0x17F => LATIN_EXTENDED_A[point - 0x100],
        _ if character.is_ascii_alphanumeric() => return Some(character.to_ascii_lowercase().to_string()),
        _ => "",
    };
    (!spelled.is_empty()).then(|| spelled.to_string())
}

/// The slug of a title: its letters and digits spelled in plain lowercase ASCII, with every run
/// of other characters between two of them written as one `-`.
pub fn slug(title: &str) -> String {
    let mut slug = String::new();
    let mut gap = false;
    for character in title.chars() {
        match plain(character) {
            Some(spelled) => {
                if gap && !slug.is_empty() {
                    slug.push('-');
                }
                gap = false;
                slug.push_str(&spelled);
            }
            None => gap = true,
        }
    }
    slug
}
