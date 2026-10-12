//! JSONC, the JSON that `tsconfig.json` and `bun.lock` are written in: JSON with comments and
//! trailing commas.

use std::sync::OnceLock;

use serde_json::Value;

/// JSONC tokenization keeps quoted strings intact while removing comments and trailing commas.
pub fn parse(bytes: &[u8]) -> Result<Value, String> {
    static TOKENS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let tokens = TOKENS
        .get_or_init(|| {
            regex::Regex::new(r#""(?:\\.|[^"\\])*"|//[^\r\n]*|/\*[\s\S]*?\*/|,[ \t\r\n]*([}\]])"#)
                .ok()
        })
        .as_ref()
        .ok_or("the JSONC token pattern does not compile")?;
    let text = std::str::from_utf8(bytes)
        .map_err(|why| why.to_string())?
        .trim_start_matches('\u{feff}');
    let clean = tokens.replace_all(text, |found: &regex::Captures| {
        if found[0].starts_with('"') {
            found[0].to_string()
        } else {
            found.get(1).map_or(" ", |end| end.as_str()).to_string()
        }
    });
    // Removing a comment can expose a trailing comma, so run the same token pass again.
    let clean = tokens.replace_all(&clean, |found: &regex::Captures| {
        found
            .get(1)
            .map_or_else(|| found[0].to_string(), |end| end.as_str().to_string())
    });
    serde_json::from_str(&clean).map_err(|why| why.to_string())
}
