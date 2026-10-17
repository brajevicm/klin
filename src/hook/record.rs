//! The one place the hook writes a file. A record for a person or a tool, the journal and the
//! report a harness asked for, passes through `token::redact` first, so a live key that any
//! source handed klin is stored as its fingerprint. Machine state that klin reads back and
//! compares, the turn stamp, the handoff, the budget and the told marker, is written as it is:
//! a branch name that looks like a key must read back unchanged, and the one text the stamp holds
//! is redacted where its verdict is made. The `single-record-writer` conventions keep every
//! other write out of `src/hook`. Spec 16.3, ADR 0075.

use std::io::{self, Write};
use std::path::Path;

use crate::sys::token;
use crate::sys::write::{AtomicWrite, atomic_write};

/// Machine state written over `target` atomically and as it is, so a run that dies partway
/// leaves the state it found and the next event reads back what this one wrote. ADR 0041.
pub fn state(target: &Path, text: &str) -> io::Result<()> {
    atomic_write(AtomicWrite {
        target,
        bytes: text.as_bytes(),
        keep_mode_from: None,
    })
}

/// The text written over `target` in place, for a file a harness named, which may be a link
/// or sit in a directory klin cannot create a file in.
pub fn write(target: &Path, text: &str) -> io::Result<()> {
    std::fs::write(target, token::redact(text).as_bytes())
}

/// The text added at the end of `target`, which it creates when absent. The journal is
/// append-only, so this is not a replacement.
pub fn append(target: &Path, text: &str) -> io::Result<()> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)?
        .write_all(token::redact(text).as_bytes())
}
