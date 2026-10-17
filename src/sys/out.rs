//! The one place klin writes to its standard output and standard error. Every text passes
//! through `token::redact` first, so a live key that any source handed klin, a line of the tree,
//! a file name, `klin.json`, a build's output or a hook event, reaches no terminal, transcript or
//! host response. The `single-output-boundary` conventions keep every other print out of
//! `src`. Spec 16.3.

use crate::sys::token;

/// The text on standard output.
pub fn print(text: &str) {
    print!("{}", token::redact(text));
}

/// The text and a newline on standard output.
pub fn println(text: &str) {
    println!("{}", token::redact(text));
}

/// The text on standard error.
pub fn eprint(text: &str) {
    eprint!("{}", token::redact(text));
}

/// The text and a newline on standard error.
pub fn eprintln(text: &str) {
    eprintln!("{}", token::redact(text));
}
