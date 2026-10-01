use std::hash::{DefaultHasher, Hash, Hasher};

use serde_json::{Map, Value};

pub type Values = Map<String, Value>;

/// The key the cross-file pass of spec 4.4 matches on: everything below the first line, with
/// every run of whitespace collapsed. Dropping that line drops the name, so a rename matches
/// too, and collapsing whitespace means a move that reindents matches. A declaration that fits
/// on one line has nothing below it, so the whole line is its key. A declaration that wraps
/// over several lines keeps its later lines, like the declaration line of ADR 0008 does.
pub fn body_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    let body = source.split_once('\n').map_or(source, |(_, body)| body);
    for word in body.split_whitespace() {
        word.hash(&mut hasher);
    }
    hasher.finish()
}
