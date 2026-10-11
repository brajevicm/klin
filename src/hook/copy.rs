//! The files `cp` and `install` write: their destination, never their sources. An option the
//! reader does not know leaves the destination unproven, so it gives up rather than guess.
//! ADR 0068.

use std::path::Path;

/// The options one of the two commands takes, as GNU and BSD both spell them. A letter the two
/// spell differently is in neither list, so it gives the destination up.
pub struct Spec {
    flags: &'static str,
    valued: &'static str,
    long: &'static [&'static str],
    long_valued: &'static [&'static str],
    /// The letter that makes every operand a directory to create, which `install` has.
    directories: Option<char>,
}

pub const CP: Spec = Spec {
    flags: "abcdfHilLnNPpRrsTuvxXZ",
    valued: "t",
    long: &[
        "archive",
        "attributes-only",
        "backup",
        "copy-contents",
        "debug",
        "dereference",
        "force",
        "interactive",
        "link",
        "no-clobber",
        "no-dereference",
        "preserve",
        "parents",
        "recursive",
        "reflink",
        "remove-destination",
        "strip-trailing-slashes",
        "symbolic-link",
        "no-target-directory",
        "update",
        "verbose",
        "one-file-system",
        "context",
        "keep-directory-symlink",
    ],
    long_valued: &["no-preserve", "sparse", "suffix", "target-directory"],
    directories: None,
};

pub const INSTALL: Spec = Spec {
    flags: "bcCdDMpsUvZ",
    valued: "BfglmNot",
    long: &[
        "backup",
        "compare",
        "debug",
        "directory",
        "preserve-timestamps",
        "strip",
        "no-target-directory",
        "verbose",
        "preserve-context",
        "context",
    ],
    long_valued: &[
        "group",
        "mode",
        "owner",
        "strip-program",
        "suffix",
        "target-directory",
    ],
    directories: Some('d'),
};

impl Spec {
    /// A long option's name and value, and `None` for one the command does not take, which
    /// includes an abbreviation.
    fn long_option<'a>(
        &self,
        option: &'a str,
        words: &mut impl Iterator<Item = &'a str>,
    ) -> Option<(&'a str, Option<&'a str>)> {
        let (name, attached) = match option.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (option, None),
        };
        let valued = self.long_valued.contains(&name);
        if !valued && !self.long.contains(&name) {
            return None;
        }
        match (valued, attached) {
            (true, None) => Some((name, Some(words.next()?))),
            _ => Some((name, attached)),
        }
    }
}

/// The paths the command writes, each spelled from its destination so a reason can quote it.
/// `is_dir` says whether a destination names a directory that exists.
pub fn destinations(spec: &Spec, args: &[String], is_dir: impl Fn(&str) -> bool) -> Vec<String> {
    let Some(read) = Read::of(spec, args) else {
        return Vec::new();
    };
    if read.directories {
        return read.operands.iter().map(|path| path.to_string()).collect();
    }
    if let Some(dir) = read.into {
        return inside(dir, &read.operands);
    }
    let Some((destination, sources)) = read.operands.split_last() else {
        return Vec::new();
    };
    match sources {
        [] => Vec::new(),
        [_] if read.file || !is_dir(destination) => vec![destination.to_string()],
        _ if read.file => Vec::new(),
        _ => inside(destination, sources),
    }
}

fn inside(dir: &str, sources: &[&str]) -> Vec<String> {
    let dir = dir.trim_end_matches('/');
    sources
        .iter()
        .filter_map(|source| Path::new(source).file_name()?.to_str())
        .map(|name| format!("{dir}/{name}"))
        .collect()
}

#[derive(Default)]
struct Read<'a> {
    operands: Vec<&'a str>,
    /// The directory `-t` names, which takes every operand as a source.
    into: Option<&'a str>,
    /// `-T`: the destination is a file even when a directory of that name exists.
    file: bool,
    directories: bool,
}

impl<'a> Read<'a> {
    fn of(spec: &Spec, args: &'a [String]) -> Option<Read<'a>> {
        let mut read = Read::default();
        let mut words = args.iter().map(String::as_str);
        while let Some(word) = words.next() {
            match word {
                "--" => read.operands.extend(words.by_ref()),
                _ if word.starts_with("--") => read.long(spec, &word[2..], &mut words)?,
                _ if word.starts_with('-') && word.len() > 1 => {
                    read.short(spec, &word[1..], &mut words)?
                }
                _ => read.operands.push(word),
            }
        }
        Some(read)
    }

    fn long(
        &mut self,
        spec: &Spec,
        option: &'a str,
        words: &mut impl Iterator<Item = &'a str>,
    ) -> Option<()> {
        let (name, value) = spec.long_option(option, words)?;
        match name {
            "target-directory" => self.into = value,
            "no-target-directory" => self.file = true,
            "directory" => self.directories = spec.directories.is_some(),
            _ => {}
        }
        Some(())
    }

    fn short(
        &mut self,
        spec: &Spec,
        cluster: &'a str,
        words: &mut impl Iterator<Item = &'a str>,
    ) -> Option<()> {
        for (at, letter) in cluster.char_indices() {
            if spec.valued.contains(letter) {
                let rest = &cluster[at + letter.len_utf8()..];
                let value = if rest.is_empty() { words.next()? } else { rest };
                if letter == 't' {
                    self.into = Some(value);
                }
                return Some(());
            }
            if !spec.flags.contains(letter) {
                return None;
            }
            self.file |= letter == 'T';
            self.directories |= spec.directories == Some(letter);
        }
        Some(())
    }
}
