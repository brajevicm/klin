#![allow(dead_code)]

//! The sources these tests plant in a temporary tree. They live under `fixtures/`, which the
//! escapes gate skips, so a test's own data is not an escape site in this repository.

pub const ONE: &str = "a.unwrap();\n";
pub const TWO_ON_TWO_LINES: &str = "a.unwrap();\na.unwrap();\n";
pub const ONE_PADDED: &str = "a.unwrap();\nfn pad() {}\n";
pub const TWO_PADDED: &str = "a.unwrap();\nfn pad() {}\na.unwrap();\n";
pub const DOUBLED: &str = "a.unwrap().unwrap();\n";
pub const DOUBLED_PADDED: &str = "a.unwrap().unwrap();\nfn pad() {}\n";
pub const DOUBLED_TWICE: &str = "a.unwrap().unwrap();\nfn pad() {}\na.unwrap().unwrap();\n";
pub const WRAPPED: &str = "fn f() {\n    a.unwrap();\n}\n";
pub const TWO_KINDS: &str = "a.unwrap(); b.expect(\"x\");\n";
pub const TWO_FILES: &str = "a.unwrap();\nb.unwrap();\n";
pub const OTHER: &str = "b.unwrap();\n";
pub const FIXED_AND_BOTH: &str = "fn fixed() {}\na.unwrap();\nb.unwrap();\n";

/// The line an accepted entry names, without the surrounding source.
pub const ONE_SITE: &str = "a.unwrap();";
pub const DOUBLED_SITE: &str = "a.unwrap().unwrap();";
