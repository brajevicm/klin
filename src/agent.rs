//! `klin __agent event [--host NAME]`, the one hidden ingress every host integration runs. It
//! places the event's host and kind before it loads configuration or walks the tree, and it
//! exits 2 only to block a Stop or to deny a tool call. Spec 10.1, 10.2, 10.10.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::check::catalogue;
use crate::config::Discovered;
use crate::host;
use crate::host::adapter::{Decision, Event, Kind};
use crate::{gate, guard, journal, turn};

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
    let placed = Placed::default();
    match catch_unwind(AssertUnwindSafe(|| answer(&placed))) {
        Ok(code) => code,
        Err(_) => {
            let kind = placed.kind.get();
            noted(
                placed.start.take().as_deref(),
                kind,
                "klin failed while it answered this hook event, so it answered nothing",
            );
            failed(kind)
        }
    }
}

/// What the answer placed before it could fail: the event's kind and the directory it reads.
#[derive(Default)]
struct Placed {
    kind: Cell<Option<Kind>>,
    start: RefCell<Option<PathBuf>>,
}

/// The event's answer. A protocol version klin does not speak is refused whole and first,
/// wherever it runs: its payload names no tree klin can trust, so it cannot opt out. Every other
/// event is answered once the opt-in walk found the worktree root's `klin.json`.
/// Spec 5.1, 10.2, 10.9.
fn answer(placed: &Placed) -> ExitCode {
    let event = match invoked() {
        Ok(Some(event)) => event,
        Ok(None) => return ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("klin: NOTE: {why}.");
            noted(std::env::current_dir().ok().as_deref(), None, &why);
            return ExitCode::SUCCESS;
        }
    };
    placed.kind.set(event.kind);
    let start = event.root.clone().or_else(|| std::env::current_dir().ok());
    placed.start.replace(start.clone());
    if event.host.refuses() {
        return ExitCode::from(refused(&event, start.as_deref()));
    }
    let Some(start) = start else {
        eprintln!(
            "klin: NOTE: the working directory could not be read, so this hook answered nothing."
        );
        return failed(event.kind);
    };
    let found = Discovered::from(&start);
    let (Some(kind), Some(root), Some(_)) = (event.kind, &found.root, &found.config) else {
        if event.kind == Some(Kind::Session)
            && let Some(nested) = found.ignored.first()
        {
            event.host.tell(&moved(nested, found.root.as_deref()));
        }
        return ExitCode::SUCCESS;
    };
    ExitCode::from(answered(event, kind, &start, root))
}

/// The refusal of a protocol version klin does not speak, which fails closed even where no tree
/// can be read. In an opted-in tree the guard delivers and journals it; anywhere else the adapter
/// delivers it alone. Spec 10.9.
fn refused(event: &Event, start: Option<&Path>) -> u8 {
    let found = start.map(Discovered::from);
    match found.as_ref().map(|found| (&found.root, &found.config)) {
        Some((Some(root), Some(_))) => guard::run(event, root),
        _ => event.host.decide(&Decision::Allow),
    }
}

/// The event this invocation carries, when it is one klin answers: `event` was named, the
/// payload reads, and the host names a kind for it or refuses it whole. An event no hook of
/// klin's runs on is none, and a usage error or a payload klin cannot read is the failure the
/// caller notes. Spec 10.1, 10.10.
fn invoked() -> Result<Option<Event>, String> {
    let words: Vec<OsString> = std::env::args_os().skip(2).collect();
    if words.first().is_none_or(|word| word != EVENT) {
        return Err(format!(
            "`klin {WORD}` takes `{EVENT}`, so this hook answered nothing"
        ));
    }
    let Some(event) = host::read(named_host(&words[1..]).as_deref()) else {
        return Err("klin could not read this hook event, so it answered nothing".to_string());
    };
    Ok((event.kind.is_some() || event.host.refuses()).then_some(event))
}

/// The journal note of an ingress that failed without a decision, in a tree that opted in and
/// whose state directory allows one. Spec 10.10.
fn noted(start: Option<&Path>, kind: Option<Kind>, message: &str) {
    let Some(start) = start else {
        return;
    };
    let found = Discovered::from(start);
    if let (Some(root), Some(_)) = (&found.root, &found.config) {
        journal::failed(root, kind.map(Kind::name), message);
    }
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
