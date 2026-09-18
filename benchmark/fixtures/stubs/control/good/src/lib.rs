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

/// Pad every line so it sits in the middle of a field `width` characters wide.
pub fn center(text: &str, width: usize) -> String {
    text.lines()
        .map(|line| {
            let room = width.saturating_sub(line.chars().count());
            " ".repeat(room / 2) + line
        })
        .collect::<Vec<_>>()
        .join("\n")
}
