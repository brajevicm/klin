//! The files `cp` and `install` write: their destination, never their sources. An option the
//! reader does not know leaves the destination unproven, so it gives up rather than guess.
//! ADR 0068.

use std::fs::Metadata;
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
        "update",
        "no-target-directory",
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
/// `look` finds what a path names on disk. Under `-n` or `-u` an existing destination may be
/// left as it is, so only one that is not there yet is a proven write. A source that names a
/// directory's contents (`dir/.`, `dir/`) writes names the command does not show, so the one
/// path it proves is the destination directory that every one of them lands under.
pub fn destinations(
    spec: &Spec,
    args: &[String],
    look: impl Fn(&str) -> Option<Metadata>,
) -> Vec<String> {
    let Some(read) = Operands::of(spec, args) else {
        return Vec::new();
    };
    if read.contents() {
        return read.destination().map(str::to_owned).into_iter().collect();
    }
    let mut written = named(&read, |path| look(path).is_some_and(|found| found.is_dir()));
    if read.no_clobber || read.update {
        written.retain(|path| look(path).is_none());
    }
    written
}

fn named(read: &Operands, is_dir: impl Fn(&str) -> bool) -> Vec<String> {
    if read.directories {
        return read.operands.iter().map(|path| path.to_string()).collect();
    }
    if let Some(dir) = read.into {
        return inside(dir, &read.operands, read.parents);
    }
    let Some((destination, sources)) = read.operands.split_last() else {
        return Vec::new();
    };
    match sources {
        [] => Vec::new(),
        [_] if read.single_file(is_dir(destination)) => vec![destination.to_string()],
        _ if read.file => Vec::new(),
        _ => inside(destination, sources, read.parents),
    }
}

/// Each source's place inside `dir`: its name, or under `--parents` its whole path.
fn inside(dir: &str, sources: &[&str], parents: bool) -> Vec<String> {
    let dir = dir.trim_end_matches('/');
    sources
        .iter()
        .filter_map(|source| match parents {
            true => Some(source.trim_start_matches('/')),
            false => Path::new(source).file_name()?.to_str(),
        })
        .map(|name| format!("{dir}/{name}"))
        .collect()
}

#[derive(Default)]
struct Operands<'a> {
    operands: Vec<&'a str>,
    /// The directory `-t` names, which takes every operand as a source.
    into: Option<&'a str>,
    /// `-T`: the destination is a file even when a directory of that name exists.
    file: bool,
    directories: bool,
    /// `--parents`: each source keeps its path under the destination directory.
    parents: bool,
    /// `-n` after any `-i`: an existing destination is left as it is.
    no_clobber: bool,
    /// `-u`, or `--update` short of `all`: an existing destination may be left as it is.
    update: bool,
}

/// The long options that mean a letter of the short form.
const LONG_LETTERS: &[(&str, char)] = &[
    ("no-target-directory", 'T'),
    ("no-clobber", 'n'),
    ("interactive", 'i'),
    ("force", 'f'),
];

impl<'a> Operands<'a> {
    /// A single source is written to the destination itself, unless the destination is a
    /// directory, `-T` aside, or `--parents` places it inside.
    fn single_file(&self, destination_is_dir: bool) -> bool {
        !self.parents && (self.file || !destination_is_dir)
    }

    /// The directory `-t` names, or else the last operand.
    fn destination(&self) -> Option<&'a str> {
        self.into.or_else(|| self.operands.last().copied())
    }

    fn contents(&self) -> bool {
        let sources = match self.into {
            Some(_) => &self.operands[..],
            None => self
                .operands
                .split_last()
                .map_or(&[][..], |(_, sources)| sources),
        };
        sources.iter().any(|source| {
            matches!(*source, "." | "..")
                || source.ends_with('/')
                || source.ends_with("/.")
                || source.ends_with("/..")
        })
    }

    fn of(spec: &Spec, args: &'a [String]) -> Option<Operands<'a>> {
        let mut read = Operands::default();
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
            "directory" => self.directories = true,
            "parents" => self.parents = true,
            "update" => self.update = keeps_existing(value)?,
            _ => short_form(name).map_or(Some(()), |letter| self.flag(letter))?,
        }
        Some(())
    }

    /// What one flag letter does. `-n` and `-i` each override the one before. `-f` after `-n`
    /// is ignored by GNU and wins on BSD, so that pair gives the destination up.
    fn flag(&mut self, letter: char) -> Option<()> {
        match letter {
            'T' => self.file = true,
            'n' => self.no_clobber = true,
            'i' => self.no_clobber = false,
            'f' if self.no_clobber => return None,
            'u' => self.update = true,
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
            self.directories |= spec.directories == Some(letter);
            self.flag(letter)?;
        }
        Some(())
    }
}

/// Whether an `--update` value may leave an existing destination as it is, and `None` for a
/// value the reader does not know.
fn keeps_existing(value: Option<&str>) -> Option<bool> {
    match value {
        None | Some("older" | "none" | "none-fail") => Some(true),
        Some("all") => Some(false),
        Some(_) => None,
    }
}

/// The short letter a long option means, when it has one.
fn short_form(name: &str) -> Option<char> {
    LONG_LETTERS
        .iter()
        .find(|(long, _)| *long == name)
        .map(|&(_, letter)| letter)
}
