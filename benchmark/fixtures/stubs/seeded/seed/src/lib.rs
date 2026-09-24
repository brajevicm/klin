//! Plain-text layout helpers.

pub mod table;

/// Put `spaces` spaces in front of every line.
pub fn indent(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Drop the whitespace at the end of every line.
pub fn strip_trailing(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Break `text` into lines of at most `width` characters.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        for part in break_word(word, width) {
            let room = width - line.chars().count();
            let needed = part.chars().count() + usize::from(!line.is_empty());
            if !line.is_empty() && needed > room {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&part);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// One word as the parts it occupies. A word wider than `width` is broken into parts of
/// `width - 1` characters, each followed by a hyphen, and the rest of the word.
fn break_word(word: &str, width: usize) -> Vec<String> {
    let letters: Vec<char> = word.chars().collect();
    if width == 0 || letters.len() <= width {
        return vec![word.to_string()];
    }
    todo!()
}
