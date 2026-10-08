//! `klin __agent event [--host NAME]`, the one hidden ingress every host integration runs. It
//! places the event's host and kind before it loads configuration or walks the tree, and it
//! exits 2 only to block a Stop or to deny a tool call. Spec 10.1, 10.2, 10.10.

use std::cell::Cell;
use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::process::ExitCode;

use crate::check::catalogue;
use crate::config::Discovered;
use crate::host;
use crate::host::adapter::{Event, Kind};
use crate::{gate, guard, turn};

const WORD: &str = "__agent";
const EVENT: &str = "event";

/// Whether this invocation is the ingress, which answers before the public command line is
/// parsed, so no argument it does not know becomes a usage error with exit 2.
pub fn called() -> bool {
    std::env::args_os().nth(1).is_some_and(|word| word == WORD)
}

/// Every answer runs under one unwind guard, so a panic anywhere is no decision either: exit 0
/// before the kind is known and at an informing hook, and exit 1 at a Stop. Spec 10.10.
pub fn run() -> ExitCode {
    let placed = Cell::new(None);
    match catch_unwind(AssertUnwindSafe(|| answer(&placed))) {
        Ok(code) => code,
        Err(_) => failed(placed.get()),
    }
}

/// The event's answer once the opt-in walk found the worktree root's `klin.json`. A protocol
/// version klin does not speak is refused whole: the guard delivers the adapter's refusal and
/// journals it, whatever kind the event meant. Spec 10.2, 10.9.
fn answer(placed: &Cell<Option<Kind>>) -> ExitCode {
    let Some(event) = invoked() else {
        return ExitCode::SUCCESS;
    };
    placed.set(event.kind);
    let Some(start) = event.root.clone().or_else(|| std::env::current_dir().ok()) else {
        eprintln!(
            "klin: NOTE: the working directory could not be read, so this hook answered nothing."
        );
        return failed(event.kind);
    };
    let found = Discovered::from(&start);
    let (Some(root), Some(_)) = (&found.root, &found.config) else {
        if event.kind == Some(Kind::Session)
            && let Some(nested) = found.ignored.first()
        {
            event.host.tell(&moved(nested, found.root.as_deref()));
        }
        return ExitCode::SUCCESS;
    };
    match (event.host.refuses(), event.kind) {
        (true, _) => ExitCode::from(guard::run(&event, root)),
        (false, Some(kind)) => ExitCode::from(answered(event, kind, &start, root)),
        (false, None) => ExitCode::SUCCESS,
    }
}

/// The event this invocation carries, when it is one klin answers: `event` was named, the
/// payload reads, and the host names a kind for it or refuses it whole. Each miss says why on
/// stderr, except an event no hook of klin's runs on. Spec 10.1, 10.10.
fn invoked() -> Option<Event> {
    let words: Vec<OsString> = std::env::args_os().skip(2).collect();
    if words.first().is_none_or(|word| word != EVENT) {
        eprintln!("klin: NOTE: `klin {WORD}` takes `{EVENT}`, so this hook answered nothing.");
        return None;
    }
    let Some(event) = host::read(named_host(&words[1..]).as_deref()) else {
        eprintln!("klin: NOTE: klin could not read this hook event, so it answered nothing.");
        return None;
    };
    (event.kind.is_some() || event.host.refuses()).then_some(event)
}

/// The value of `--host NAME` or `--host=NAME`. Every other argument is one the ingress does
/// not know, which it ignores. Spec 10.1.
fn named_host(words: &[OsString]) -> Option<String> {
    let words: Vec<String> = words
        .iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect();
    words
        .iter()
        .enumerate()
        .find_map(|(at, word)| match word.strip_prefix("--host=") {
            Some(named) => Some(named.to_string()),
            None => (word == "--host")
                .then(|| words.get(at + 1).cloned())
                .flatten(),
        })
}

fn answered(event: Event, kind: Kind, start: &Path, root: &Path) -> u8 {
    let mut out = String::new();
    let code = match kind {
        Kind::PreTool => guard::run(&event, root),
        Kind::Session | Kind::Prompt => {
            turn::opened(event, start, &catalogue::sections(), &mut out)
        }
        Kind::Stop => gate::stop(event, start, &mut out),
    };
    print!("{out}");
    code
}

/// A failure that is no decision: exit 0 where a hook only informs, and exit 1 at a Stop, so
/// no host reads it as a block or a deny. Spec 10.10.
fn failed(kind: Option<Kind>) -> ExitCode {
    match kind {
        Some(Kind::Stop) => ExitCode::from(1),
        _ => ExitCode::SUCCESS,
    }
}

/// The notice that a `klin.json` below the worktree root turns nothing on. Spec 5.1.
fn moved(nested: &Path, root: Option<&Path>) -> String {
    let root = root.map_or_else(String::new, |root| format!(" {}", root.display()));
    format!(
        "klin: {} is not read, so klin is off here. klin reads only the klin.json at the \
         worktree root{root}. Move the file there to turn klin on.",
        nested.display()
    )
}
