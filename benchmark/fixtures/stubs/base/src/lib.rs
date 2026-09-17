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
