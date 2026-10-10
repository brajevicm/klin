use std::fmt::Write;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::file as config;
use crate::hook::host::ADAPTERS;
use crate::hook::host::adapter::{Adapter, Filter, Hook, HookFile, Kind};
use crate::sys::error::Error;
use crate::{
    cli::starter,
    sys::{state, write},
};

const HOOKS: &str = "hooks";
const MARKER: &str = "klin.json";
const SKILL: &str = include_str!("../../plugins/klin/skills/klin/SKILL.md");
/// The `state::hash` of every earlier text of `SKILL`. A change to the skill adds the hash of
/// the text it replaced, or the status test that reads git history fails. A file that holds one
/// of them is klin's, written by an earlier `setup` and changed by nobody. Spec B.19.3.
const EARLIER_SKILLS: &[u64] = &[
    0x16e8_4f5d_63b9_eb01,
    0x5c53_ac81_e30d_2e9b,
    0x38ce_1765_4060_07d6,
    0x2392_3714_0eaf_6407,
    0xb3f2_7336_a44d_2718,
    0x326d_c80b_49d4_dee2,
    0x3f39_2001_cec3_bf63,
    0x82ce_b777_ce4c_3978,
    0x234d_4187_3cef_7ffb,
    0x51c1_5f02_ab11_3294,
];

/// The klin commands a host hook runs: the ingress, and the 0.x commands it replaced. An entry
/// that runs one of them is klin's own, whatever shape the klin that wrote it used, so `setup`
/// replaces an old line. An entry that runs another klin command is a person's, and this command
/// leaves it where it is. Spec 11.2.
const LIFECYCLE: &[&str] = &["__agent", "radius", "guard", "gate"];
/// What every hook line klin writes runs. Spec 10.1.
const INGRESS: &str = "__agent event";

const NO_REPOSITORY: &str = "klin setup writes a repository's own files, and this is no git \
    repository. Run it inside one, or run klin setup --user --host NAME to set up the host \
    files of one person on this machine.";

const NO_HOME: &str = "--user writes the host files of one person on this machine, and this \
    system names no home directory.";

#[derive(clap::Args)]
pub struct Args {
    /// The host to install for, named again for a second one (default: every host this
    /// repository proves, or all of them where it proves none)
    #[arg(long = "host")]
    hosts: Vec<String>,
    /// Install into the host files of one person on this machine, rather than this
    /// repository's own
    #[arg(long)]
    user: bool,
    /// Write today's complexity ceilings, document ceilings and change radius into the
    /// configuration as policy, and keep every value it already holds
    #[arg(long)]
    pin: bool,
    /// The klin.json to write, which must be the one at the worktree root (the default)
    #[arg(long)]
    config: Option<PathBuf>,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let scope = Scope::of(args.user, start)?;
    let file = configuration(args, &scope, start)?;
    let components = planned(args, &scope, file.as_deref())?;
    if let Some(file) = file.as_deref().filter(|_| args.pin) {
        starter::pin(file, out)?;
    }
    applied(&components, out)?;
    if let Some(root) = &scope.repository {
        starter::inert(root, out);
    }
    let _ = writeln!(out, "{}", scope.closing(hooks_written(&components)));
    Ok(0)
}

/// Whether two paths name one file, through the directories that hold them, so a file that does
/// not exist yet still compares.
fn same_file(one: &Path, other: &Path) -> bool {
    let held = |path: &Path| {
        let directory = path.parent()?.canonicalize().ok()?;
        Some(directory.join(path.file_name()?))
    };
    held(one).is_some_and(|one| held(other) == Some(one))
}

/// The configuration this run opts in and `--pin` writes, refused before anything is written
/// where `--config` names a file the hooks never read, or `--pin` has no repository to name one.
fn configuration(args: &Args, scope: &Scope, start: &Path) -> Result<Option<PathBuf>, Error> {
    let file = scope.config(args.config.as_deref(), start)?;
    pin_target(args.pin, file.as_deref())?;
    Ok(file)
}

/// The configuration `--pin` writes, which needs a repository to name it.
fn pin_target(pin: bool, file: Option<&Path>) -> Result<Option<&Path>, Error> {
    match (pin, file) {
        (false, _) => Ok(None),
        (true, Some(file)) => Ok(Some(file)),
        (true, None) => Err(Error(NO_REPOSITORY.to_string())),
    }
}

/// Whether the run wrote a host's file. The repository marker is written on its own account,
/// and a run that wrote that alone installed no hooks, so the closing line must not speak of
/// hooks a teammate will find.
fn hooks_written(components: &[Component]) -> bool {
    components
        .iter()
        .filter(|component| component.host)
        .any(|component| !component.targets.is_empty())
}

/// Where one run installs: the directory whose host files it writes, and the repository it
/// opts in. A user-scope run inside a repository opts that repository in and writes the
/// person's own host files; it never writes a configuration beside the home directory.
/// Section 19.3.
struct Scope {
    at: PathBuf,
    repository: Option<PathBuf>,
    user: bool,
}

impl Scope {
    fn of(user: bool, start: &Path) -> Result<Scope, Error> {
        let repository = config::repository(start);
        let at = match user {
            true => std::env::home_dir().ok_or_else(|| Error(NO_HOME.to_string()))?,
            false => repository
                .clone()
                .ok_or_else(|| Error(NO_REPOSITORY.to_string()))?,
        };
        Ok(Scope {
            at,
            repository,
            user,
        })
    }

    /// The configuration this run opts in: the repository's own. The hooks read only the
    /// `klin.json` at the worktree root, so a `--config` that names any other file would report
    /// an opt-in no hook ever sees, and is refused. Spec 5.1, 11.2.
    fn config(&self, named: Option<&Path>, start: &Path) -> Result<Option<PathBuf>, Error> {
        let own = self.repository.as_ref().map(|root| root.join(MARKER));
        let Some(named) = named.map(|named| start.join(named)) else {
            return Ok(own);
        };
        match own.as_deref().is_some_and(|own| same_file(own, &named)) {
            true => Ok(own),
            false => Err(Error(format!(
                "--config names {}, and the hooks read only the klin.json at the worktree root{}. \
                 Run klin setup without --config; klin check --config still reads that file.",
                named.display(),
                own.map_or_else(String::new, |own| format!(", {}", own.display()))
            ))),
        }
    }

    /// The person's own file for this host, when it already holds klin's entries beside the
    /// repository's. A host reads both files and runs both copies, so on this machine one of
    /// them yields on each event. Spec 9.8.
    fn covered_by_user(&self, host: &dyn Adapter) -> Option<PathBuf> {
        if self.user {
            return None;
        }
        let file = std::env::home_dir()?.join(host.hook_file());
        holds_klin(&file).then_some(file)
    }

    /// What the file this run writes is called, beside another copy of klin's hooks.
    fn copy(&self) -> &'static str {
        match self.user {
            true => "the copy klin wrote",
            false => "the committed copy",
        }
    }

    /// The last line: what the host files klin wrote mean, or that there were none to write.
    fn closing(&self, wrote_hooks: bool) -> &'static str {
        match (wrote_hooks, self.user) {
            (false, _) => "The integration this run selected is already current.",
            (true, true) => {
                "These host files cover every repository you open on this machine. They are \
                 not committed, they do not travel with a repository, and they do not reach a \
                 cloud or remote agent."
            }
            (true, false) => {
                "Commit the host files klin wrote, so a teammate who clones the repository \
                 gets the hooks."
            }
        }
    }
}

/// Everything the run will do, resolved before it writes anything: the repository marker, and
/// one component per host klin knows. A host file that cannot be read or is not the shape the
/// host reads fails here, where no file has been touched yet. Section 19.3.
/// The configuration is planned only where `--pin` does not write it itself.
fn planned(args: &Args, scope: &Scope, file: Option<&Path>) -> Result<Vec<Component>, Error> {
    let (wanted, why) = chosen(args, scope)?;
    let mut skills = Vec::new();
    let mut components = match args.pin {
        true => Vec::new(),
        false => vec![opt_in(file)],
    };
    components.extend(why);
    for host in ADAPTERS.iter().copied() {
        let named = wanted.iter().any(|one| one.name() == host.name());
        components.push(match named {
            true => component(host, scope, &mut skills)?,
            false => told(host, "no integration requested.".to_string()),
        });
    }
    Ok(components)
}

/// One thing the run does: the line it prints, and the file it writes when the thing is not
/// already so. A component with no file is already as it should be.
struct Component {
    said: String,
    targets: Vec<Target>,
    /// Whether this is a host's integration, rather than the repository's own marker.
    host: bool,
}

struct Target {
    file: PathBuf,
    bytes: Vec<u8>,
    /// The file a person agreed to replace, which the write must still reach. Section B.19.3.
    agreed: Option<PathBuf>,
}

fn told(host: &dyn Adapter, said: String) -> Component {
    Component {
        said: format!("{}: {said}", host.name()),
        targets: Vec::new(),
        host: true,
    }
}

/// Every component in turn, each written before its line is printed, so the text says what
/// happened and not what was planned. A write that fails stops the run and names the files it
/// never reached, because a partial install read as a complete one leaves a host ungated.
fn applied(components: &[Component], out: &mut String) -> Result<(), Error> {
    for (at, component) in components.iter().enumerate() {
        for (target_at, target) in component.targets.iter().enumerate() {
            if let Err(why) = written(target) {
                incomplete(components, at, target_at, out);
                return Err(why);
            }
        }
        let _ = writeln!(out, "{}", component.said);
    }
    Ok(())
}

fn written(target: &Target) -> Result<(), Error> {
    if let Some(agreed) = &target.agreed
        && resolved(&target.file).as_ref().ok() != Some(agreed)
    {
        return Err(Error(format!(
            "{} no longer reaches {}, the file setup planned to write, so klin did not write it",
            target.file.display(),
            agreed.display()
        )));
    }
    if let Some(parent) = target.file.parent() {
        std::fs::create_dir_all(parent).map_err(|why| Error::unreadable(parent, why))?;
    }
    let held = std::fs::canonicalize(&target.file);
    let path = held.as_deref().unwrap_or(&target.file);
    write::atomic_write(write::AtomicWrite {
        target: path,
        bytes: &target.bytes,
        keep_mode_from: Some(path),
    })
    .map_err(|why| {
        Error(format!(
            "{} could not be written: {why}",
            target.file.display()
        ))
    })
}

fn incomplete(components: &[Component], component_at: usize, target_at: usize, out: &mut String) {
    let written: Vec<String> = components[..component_at]
        .iter()
        .flat_map(|component| component.targets.iter())
        .chain(components[component_at].targets[..target_at].iter())
        .map(|target| target.file.display().to_string())
        .collect();
    let not_written: Vec<String> = components[component_at..]
        .iter()
        .enumerate()
        .flat_map(|(at, component)| {
            let start = if at == 0 { target_at } else { 0 };
            component.targets[start..]
                .iter()
                .map(|target| target.file.display().to_string())
        })
        .collect();
    let _ = write!(
        out,
        "incomplete: klin did not write {}",
        not_written.join(", ")
    );
    if !written.is_empty() {
        let _ = writeln!(out, "; it wrote {}.", written.join(", "));
    } else {
        let _ = writeln!(out, ".");
    }
}

/// The repository's opt-in marker, at the repository root and not at the directory the command
/// was run from. A marker a person already wrote is theirs, and its content is kept whole.
/// Spec 5.1, ADR 0028.
fn opt_in(file: Option<&Path>) -> Component {
    let Some(file) = file else {
        return Component {
            said: "klin: no repository was opted in, because this is no git repository."
                .to_string(),
            targets: Vec::new(),
            host: false,
        };
    };
    match file.is_file() {
        true => Component {
            said: format!("klin: {} already opts this repository in.", file.display()),
            targets: Vec::new(),
            host: false,
        },
        false => Component {
            said: format!(
                "klin: repository opted in at {} — commit it, so the repository stays opted in \
                 for everyone. klin setup --pin writes today's ceilings into it as policy a \
                 person reviews.",
                file.display()
            ),
            targets: vec![Target {
                file: file.to_path_buf(),
                bytes: b"{}\n".to_vec(),
                agreed: None,
            }],
            host: false,
        },
    }
}

/// The hosts this run serves, and the line that says why where klin chose every one of them
/// rather than the hosts the scope proves.
fn chosen(
    args: &Args,
    scope: &Scope,
) -> Result<(Vec<&'static dyn Adapter>, Option<Component>), Error> {
    if !shows_no_host(args, scope) {
        return Ok((selected(args, scope)?, None));
    }
    let why = Component {
        said: format!(
            "klin: this repository shows no host, so every host klin knows gets its hooks — \
             name fewer with --host, one of {}.",
            names()
        ),
        targets: Vec::new(),
        host: false,
    };
    Ok((ADAPTERS.to_vec(), Some(why)))
}

/// Whether this run reconciles every host: a repository run that names none and holds no
/// host's directory. The repository serves a team whose hosts klin cannot see, and a hook file
/// for a host nobody runs does nothing. A plugin in the person's own home is no directory of the
/// repository's, so what the repository gets does not depend on who runs the install. Section
/// 19.3, ADR 0056.
fn shows_no_host(args: &Args, scope: &Scope) -> bool {
    args.hosts.is_empty()
        && !scope.user
        && !ADAPTERS
            .iter()
            .any(|host| scope.at.join(host.marker()).is_dir())
}

/// The hosts this run reconciles: the ones `--host` names, or every host the scope proves. One
/// person's home that proves none is refused rather than guessed at. Section 19.3.
fn selected(args: &Args, scope: &Scope) -> Result<Vec<&'static dyn Adapter>, Error> {
    if !args.hosts.is_empty() {
        return args.hosts.iter().map(|name| by_name(name)).collect();
    }
    let proven: Vec<&'static dyn Adapter> = ADAPTERS
        .iter()
        .copied()
        .filter(|host| provable(*host, scope))
        .collect();
    match proven.is_empty() {
        true => Err(Error(format!(
            "{}: no host klin knows is configured here, and none has klin's plugin enabled — \
             name one with --host, one of {}",
            scope.at.display(),
            names()
        ))),
        false => Ok(proven),
    }
}

/// A host this scope can prove: its own configuration directory, or klin's plugin enabled for
/// this scope. A marker directory alone is evidence of the host, never of the install.
fn provable(host: &dyn Adapter, scope: &Scope) -> bool {
    scope.at.join(host.marker()).is_dir() || host.plugin_enabled(&scope.at, scope.user).is_some()
}

fn by_name(name: &str) -> Result<&'static dyn Adapter, Error> {
    ADAPTERS
        .iter()
        .copied()
        .find(|host| host.name() == name)
        .ok_or_else(|| {
            Error(format!(
                "--host {name} names no host klin knows — name one of {}",
                names()
            ))
        })
}

fn names() -> String {
    ADAPTERS
        .iter()
        .map(|host| host.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What one selected host needs: its file with klin's entries brought to today's contract, and
/// the skill. Where a plugin or the person's own file already runs klin's hooks here, the line
/// says so: the host runs both copies on this machine, and one of them yields on each event.
/// Section 19.3, 9.8.
fn component(
    host: &'static dyn Adapter,
    scope: &Scope,
    skills: &mut Vec<PathBuf>,
) -> Result<Component, Error> {
    let beside = host
        .plugin_enabled(&scope.at, scope.user)
        .map(|proof| host.plugin_serves(&proof))
        .or_else(|| {
            scope
                .covered_by_user(host)
                .map(|user| format!("{} also holds klin's hooks", user.display()))
        });
    let mut component = reconciled(host, &scope.at.join(host.hook_file()))?;
    let (target, said) = skill(host, scope, skills)?;
    if let Some(target) = target {
        component.targets.push(target);
    }
    let said = said.into_iter().chain(beside.map(|beside| {
        format!(
            "{beside}, so on this machine {} yields on each event the other copy took first",
            scope.copy()
        )
    }));
    for said in said {
        let prefix = component.said.trim_end_matches('.');
        component.said = format!("{prefix}; {said}.");
    }
    Ok(component)
}

/// One host's file as klin would have it. A file already holding exactly that is left alone,
/// so a second complete run writes nothing at all.
fn reconciled(host: &'static dyn Adapter, file: &Path) -> Result<Component, Error> {
    let held = read(file)?;
    let settings = canonical(host, held.clone(), file)?;
    if file.is_file() && settings == held {
        return Ok(told(
            host,
            format!("{} is already current.", file.display()),
        ));
    }
    let mut bytes = serde_json::to_vec_pretty(&Value::Object(settings))
        .map_err(|why| Error(format!("{} could not be written: {why}", file.display())))?;
    bytes.push(b'\n');
    Ok(Component {
        said: format!("{}: reconciled {}.", host.name(), file.display()),
        targets: vec![Target {
            file: file.to_path_buf(),
            bytes,
            agreed: None,
        }],
        host: true,
    })
}

/// The standalone skill is one canonical text. A missing file is klin's to write, the exact
/// current text is already current, and an earlier text is klin's to replace. Any other text is
/// a person's: on a terminal the person says whether to replace it, and without one it is
/// replaced and named. A link that sends any write outside the skill's own path is followed
/// only on a yes at a terminal. Shared Codex and Cursor paths are planned once. Section B.19.3.
fn skill(
    host: &dyn Adapter,
    scope: &Scope,
    seen: &mut Vec<PathBuf>,
) -> Result<(Option<Target>, Option<String>), Error> {
    let file = scope.at.join(host.skill_file());
    if seen.iter().any(|held| held == &file) {
        return Ok((None, None));
    }
    seen.push(file.clone());
    let own = scope
        .at
        .canonicalize()
        .map_err(|why| Error::unreadable(&scope.at, why))?
        .join(host.skill_file());
    let reached = resolved(&file).map_err(|why| Error::unreadable(&file, why))?;
    let linked = (reached != own).then_some(reached.as_path());
    let Some(said) = rewritten(&file, linked)? else {
        return Ok((None, Some(format!("{} is already current", file.display()))));
    };
    let target = Target {
        file,
        bytes: SKILL.as_bytes().to_vec(),
        agreed: Some(reached),
    };
    Ok((Some(target), Some(said)))
}

/// Where a path resolves through every link, for a file that does not exist yet too: its
/// nearest existing directory resolved, and the rest of the path joined on.
fn resolved(path: &Path) -> std::io::Result<PathBuf> {
    match path.canonicalize() {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            match (path.parent(), path.file_name()) {
                (Some(parent), Some(name)) => Ok(resolved(parent)?.join(name)),
                _ => Err(why),
            }
        }
        held => held,
    }
}

/// What `setup` says it did to a skill file it writes, or `None` where the file is current. A
/// changed skill, or any write a link sends to `linked`, needs consent.
fn rewritten(file: &Path, linked: Option<&Path>) -> Result<Option<String>, Error> {
    let Some((said, differs)) = write_of(file)? else {
        return Ok(None);
    };
    if (differs || linked.is_some()) && !write_agreed(file, linked)? {
        return Err(kept(file, linked));
    }
    Ok(Some(match linked {
        Some(to) => format!("{said}, through a link to {}", to.display()),
        None => said,
    }))
}

/// The write a skill file needs, and whether it replaces a text a person may have changed, or
/// `None` where the file is current.
fn write_of(file: &Path) -> Result<Option<(String, bool)>, Error> {
    Ok(Some(match held_skill(file) {
        Ok(Held::Current) => return Ok(None),
        Ok(Held::Earlier) => (
            format!("replaced {}, an earlier klin skill", file.display()),
            false,
        ),
        Ok(Held::Other) => (
            format!(
                "replaced {}, which differed from klin's skill",
                file.display()
            ),
            true,
        ),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            (format!("wrote {}", file.display()), false)
        }
        Err(why) => return Err(Error::unreadable(file, why)),
    }))
}

/// The refusal for a skill write nobody agreed to: the run writes nothing.
fn kept(file: &Path, linked: Option<&Path>) -> Error {
    let why = match linked {
        Some(to) => format!(
            "{} reaches {} through a link, and klin writes a skill through a link only on a \
             yes at a terminal",
            file.display(),
            to.display()
        ),
        None => format!(
            "{}: kept the existing skill, which differs from klin's canonical skill",
            file.display()
        ),
    };
    Error(format!(
        "{why}. klin wrote nothing. Move the file or the link aside, or reconcile it, then \
         rerun klin setup"
    ))
}

/// Whether a person agrees to a skill write that needs consent. Without a terminal nobody can
/// answer: the skill's own file is replaced and printed, and a link is not followed.
/// Section B.19.3.
fn write_agreed(file: &Path, linked: Option<&Path>) -> Result<bool, Error> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return Ok(linked.is_none());
    }
    match linked {
        Some(to) => eprint!(
            "{} reaches {} through a link. Write klin's skill at {}? [y/N] ",
            file.display(),
            to.display(),
            to.display()
        ),
        None => eprint!(
            "{} differs from klin's canonical skill. Replace it? [y/N] ",
            file.display()
        ),
    }
    let mut answer = String::new();
    stdin
        .read_line(&mut answer)
        .map_err(|why| Error(format!("the answer could not be read: {why}")))?;
    let answer = answer.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// What a skill file holds, judged against the skill this klin writes.
enum Held {
    Current,
    Earlier,
    Other,
}

impl Held {
    /// Whether klin wrote the text, this klin or an earlier one.
    fn is_klins(&self) -> bool {
        matches!(self, Held::Current | Held::Earlier)
    }
}

/// The one place that judges a skill file, so every reader of it agrees.
fn held_skill(file: &Path) -> std::io::Result<Held> {
    let text = std::fs::read(file)?;
    Ok(if text == SKILL.as_bytes() {
        Held::Current
    } else if EARLIER_SKILLS.contains(&state::hash(&text)) {
        Held::Earlier
    } else {
        Held::Other
    })
}

/// klin's own entries brought to the current contract: one entry per event klin writes, with
/// today's command and matcher, every further entry of klin's removed, and every entry that is
/// not klin's left where it stands. Section 19.3.
fn canonical(
    host: &dyn Adapter,
    mut settings: Map<String, Value>,
    file: &Path,
) -> Result<Map<String, Value>, Error> {
    if let HookFile::Flat { version } = host.hook_file_kind() {
        settings
            .entry("version")
            .or_insert_with(|| Value::from(version));
    }
    let held = settings
        .entry(HOOKS)
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(events) = held.as_object_mut() else {
        return Err(malformed(file, HOOKS));
    };
    retired(events, host);
    for hook in host.hooks() {
        placed(events, host, hook, file)?;
    }
    Ok(settings)
}

/// Every entry of klin's on an event klin no longer writes, removed, and an event that
/// removal empties removed with it. An entry that is not klin's keeps the event alive, and an
/// event that is not a list of entries is another tool's business and is left whole.
fn retired(events: &mut Map<String, Value>, host: &dyn Adapter) {
    let written: Vec<&str> = host.hooks().iter().map(|hook| hook.event).collect();
    let stale: Vec<String> = events
        .keys()
        .filter(|event| !written.contains(&event.as_str()))
        .cloned()
        .collect();
    for event in stale {
        let emptied = match events.get_mut(&event).and_then(Value::as_array_mut) {
            Some(entries) => {
                let held = std::mem::take(entries);
                *entries = held
                    .into_iter()
                    .filter_map(|entry| without_klin(entry).0)
                    .collect();
                entries.is_empty()
            }
            None => false,
        };
        if emptied {
            events.remove(&event);
        }
    }
}

/// One canonical entry on its event: where klin's own command already stood, or at the end
/// where it stood nowhere, and every further command of klin's on that event removed.
fn placed(
    events: &mut Map<String, Value>,
    host: &dyn Adapter,
    hook: &Hook,
    file: &Path,
) -> Result<(), Error> {
    let wanted = entry(host, hook, &line(hook.kind));
    let entries = array(events, hook.event, file)?;
    let held = std::mem::take(entries);
    let mut placed = false;
    for entry in held {
        let (kept, was_klins) = without_klin(entry);
        entries.extend(kept);
        if was_klins && !placed {
            entries.push(wanted.clone());
            placed = true;
        }
    }
    if !placed {
        entries.push(wanted);
    }
    Ok(())
}

/// One entry with every command of klin's taken out, and whether it held one. A host nests
/// several commands under one entry, and a person's command may sit beside klin's there, so
/// klin takes out its own command and leaves the entry holding the rest. An entry left holding
/// no command at all goes. Section 19.3.
fn without_klin(mut entry: Value) -> (Option<Value>, bool) {
    let Some(commands) = entry.get_mut(HOOKS).and_then(Value::as_array_mut) else {
        let klins = klins(&entry);
        return ((!klins).then_some(entry), klins);
    };
    let held = commands.len();
    commands.retain(|command| !command["command"].as_str().is_some_and(runs_a_hook));
    let was_klins = commands.len() < held;
    match commands.is_empty() {
        true => (None, was_klins),
        false => (Some(entry), was_klins),
    }
}

fn array<'a>(
    events: &'a mut Map<String, Value>,
    event: &str,
    file: &Path,
) -> Result<&'a mut Vec<Value>, Error> {
    events
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| malformed(file, event))
}

/// Every line klin writes resolves the binary before it runs it, and ends the hook when none
/// resolves. A person who uninstalls klin, or installs it where the hook's shell does not look,
/// would otherwise see a failed hook on every event of every session. After PATH it looks where
/// the installer puts klin, because a host started from the terminal that ran the installer has
/// no such PATH yet. The stop of a repository
/// that opted in says how to install it instead, so a teammate who cloned the committed hooks
/// learns what they are for. It looks for the marker at the Git root, because a session may
/// start below it. It is a `systemMessage` alone: Cursor submits a `followup_message`
/// as the next prompt, which would hand the installer to the agent. Section 19.3.
fn line(kind: Kind) -> String {
    let missing = match kind == Kind::Stop {
        true => format!(
            "{{ r=$(git rev-parse --show-toplevel 2>/dev/null) && [ -f \"$r/klin.json\" ] && echo \
             '{{\"systemMessage\":\"{MISSING}\"}}'; exit 0; }}"
        ),
        false => "exit 0".to_string(),
    };
    format!(
        "PATH=\"$PATH:$HOME/.local/bin\"; command -v klin > /dev/null 2>&1 || {missing}; \
         klin {INGRESS}"
    )
}

const MISSING: &str = "klin is not installed. Install it with: curl --proto =https --tlsv1.2 \
                       -LsSf https://github.com/brajevicm/klin/releases/latest/download/\
                       klin-installer.sh | sh";

/// What one entry filters by, in the host's matcher syntax: nothing, or the tools the guard
/// reads. The hook table says which, per event.
fn matcher(host: &dyn Adapter, hook: &Hook) -> &'static str {
    match hook.filter {
        Filter::Every => "",
        Filter::Tools => host.matcher(),
    }
}

/// One entry in the host's own shape: the matcher the event filters by, when it filters, and
/// the command either beside it or under the host's nested `hooks` array.
fn entry(host: &dyn Adapter, hook: &Hook, command: &str) -> Value {
    let mut entry = Map::new();
    let matcher = matcher(host, hook);
    if !matcher.is_empty() {
        entry.insert("matcher".to_string(), matcher.into());
    }
    match host.hook_file_kind() {
        HookFile::Flat { .. } => {
            entry.insert("command".to_string(), command.into());
        }
        HookFile::Nested => {
            entry.insert(
                HOOKS.to_string(),
                serde_json::json!([{"type": "command", "command": command}]),
            );
        }
    }
    Value::Object(entry)
}

/// A host file that already holds an entry of klin's on some event.
fn holds_klin(settings: &Path) -> bool {
    let Ok(held) = read(settings) else {
        return false;
    };
    held.get(HOOKS)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(_, entries)| entries.as_array())
        .flatten()
        .any(klins)
}

/// An entry klin owns. The test is the command's own words, not the text of the entry: a hook
/// that only mentions klin, such as a script under a directory named after it, is another
/// tool's entry, and reading it as klin's leaves the event ungated.
fn klins(entry: &Value) -> bool {
    if let Some(command) = entry.get("command").and_then(Value::as_str)
        && runs_a_hook(command)
    {
        return true;
    }
    entry["hooks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|hook| hook["command"].as_str())
        .any(runs_a_hook)
}

/// Whether a command runs the klin binary on one of the commands a hook runs. The word after
/// the binary decides it, so a person's own `klin report` hook is theirs and stays.
fn runs_a_hook(command: &str) -> bool {
    let words: Vec<&str> = command
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '"']))
        .collect();
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| is_klin(word))
        .filter_map(|(at, _)| words[at + 1..].iter().find(|word| !word.starts_with('-')))
        .any(|word| LIFECYCLE.contains(word))
}

fn is_klin(word: &str) -> bool {
    word == "klin" || word.ends_with("/klin")
}

fn malformed(file: &Path, key: &str) -> Error {
    Error(format!(
        "{}: \"{key}\" is not the shape this host reads, so klin left it alone",
        file.display()
    ))
}

fn read(file: &Path) -> Result<Map<String, Value>, Error> {
    if !file.is_file() {
        return Ok(Map::new());
    }
    let text = std::fs::read_to_string(file).map_err(|why| Error::unreadable(file, why))?;
    match serde_json::from_str(&text).map_err(|why| Error::unreadable(file, why))? {
        Value::Object(held) => Ok(held),
        _ => Err(Error(format!(
            "{}: a host's settings are an object",
            file.display()
        ))),
    }
}

/// One copy of klin's integration for a host, as `klin status` reports it. Spec 11.4.
pub struct Integration {
    pub host: &'static str,
    pub scope: Owner,
    pub route: Route,
    pub state: State,
    pub detail: String,
}

/// Whose files hold a copy: the repository's, or one person's on this machine.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Project,
    User,
}

/// How a copy reaches the host: hook lines in its file, or klin's plugin.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Hooks,
    Plugin,
}

/// The integration states of spec 11.4.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Current,
    Missing,
    Conflict,
}

impl Owner {
    pub fn as_str(self) -> &'static str {
        match self {
            Owner::Project => "project",
            Owner::User => "user",
        }
    }
}

impl Route {
    pub fn as_str(self) -> &'static str {
        match self {
            Route::Hooks => "hooks",
            Route::Plugin => "plugin",
        }
    }
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Current => "current",
            State::Missing => "missing",
            State::Conflict => "conflict",
        }
    }
}

/// Every copy of klin's integration the repository at `root` and the person's home hold, and a
/// `missing` row for each host the repository proves where no copy is installed. It reads the
/// host files and writes nothing. Spec 11.4.
pub fn integrations(root: &Path) -> Vec<Integration> {
    let home = std::env::home_dir();
    let scopes: Vec<(Owner, &Path)> =
        [(Owner::Project, Some(root)), (Owner::User, home.as_deref())]
            .into_iter()
            .filter_map(|(scope, at)| Some((scope, at?)))
            .collect();
    let mut held: Vec<Vec<Vec<Integration>>> = scopes
        .iter()
        .map(|&(scope, at)| {
            ADAPTERS
                .iter()
                .map(|host| copies(*host, scope, at))
                .collect()
        })
        .collect();
    let mut alone: Vec<Vec<Option<Integration>>> = scopes
        .iter()
        .zip(&held)
        .map(|(&(scope, at), copies)| {
            ADAPTERS
                .iter()
                .map(|host| skill_alone(*host, scope, at, copies))
                .collect()
        })
        .collect();
    let mut found = Vec::new();
    for (at_host, host) in ADAPTERS.iter().copied().enumerate() {
        let mut rows: Vec<Integration> = held
            .iter_mut()
            .zip(&mut alone)
            .flat_map(|(copies, alone)| {
                std::mem::take(&mut copies[at_host])
                    .into_iter()
                    .chain(alone[at_host].take())
            })
            .collect();
        if rows.is_empty() && root.join(host.marker()).is_dir() {
            rows.push(Integration {
                host: host.name(),
                scope: Owner::Project,
                route: Route::Hooks,
                state: State::Missing,
                detail: format!(
                    "{} proves the host, and no copy of klin's hooks is installed",
                    host.marker()
                ),
            });
        }
        found.extend(rows);
    }
    found
}

/// The copies of klin's integration one scope holds for one host: its plugin, and its hook file
/// when that file holds klin's entries.
fn copies(host: &'static dyn Adapter, scope: Owner, at: &Path) -> Vec<Integration> {
    let plugin = host
        .plugin_enabled(at, scope == Owner::User)
        .filter(|proof| proof.starts_with(at))
        .map(|proof| Integration {
            host: host.name(),
            scope,
            route: Route::Plugin,
            state: State::Current,
            detail: format!("{} enables klin's plugin", proof.display()),
        });
    let file = at.join(host.hook_file());
    let hooks = holds_klin(&file).then(|| {
        let (state, detail) = hook_state(host, at, &file);
        Integration {
            host: host.name(),
            scope,
            route: Route::Hooks,
            state,
            detail,
        }
    });
    plugin.into_iter().chain(hooks).collect()
}

/// klin's skill at a scope where this host is the one host that reads it and the scope proves,
/// and no such reader holds a copy of klin's integration: an install whose hook lines are gone.
/// A skill two proven hosts share names neither, so the row never asks for a host nobody used.
/// Spec 11.4.
fn skill_alone(
    host: &'static dyn Adapter,
    scope: Owner,
    at: &Path,
    copies: &[Vec<Integration>],
) -> Option<Integration> {
    let readers = || {
        ADAPTERS
            .iter()
            .zip(copies)
            .filter(|(reader, _)| reader.skill_file() == host.skill_file())
    };
    let skill = at.join(host.skill_file());
    let alone = readers()
        .filter(|(reader, _)| at.join(reader.marker()).is_dir())
        .map(|(reader, _)| reader.name())
        .eq([host.name()])
        && readers().all(|(_, held)| held.is_empty())
        && held_skill(&skill).is_ok_and(|held| held.is_klins());
    if !alone {
        return None;
    }
    let file = at.join(host.hook_file());
    let wrong = match read(&file) {
        Ok(_) => format!("{} holds none of klin's hook lines", file.display()),
        Err(why) => why.0,
    };
    Some(Integration {
        host: host.name(),
        scope,
        route: Route::Hooks,
        state: State::Conflict,
        detail: format!("{} is klin's skill, and {wrong}", skill.display()),
    })
}

/// Whether a hook file and the skill beside it hold what this klin's `setup` writes. Only the
/// exact skill reads as current: a skill that is missing, unreadable or changed is a conflict
/// `setup` names. Spec 11.4.
fn hook_state(host: &'static dyn Adapter, at: &Path, file: &Path) -> (State, String) {
    let held = read(file).unwrap_or_default();
    let current = canonical(host, held.clone(), file).is_ok_and(|wanted| wanted == held);
    if !current {
        return (
            State::Conflict,
            format!(
                "{} holds hook lines this klin's setup does not write",
                file.display()
            ),
        );
    }
    let skill = at.join(host.skill_file());
    let wrong = match held_skill(&skill) {
        Ok(Held::Current) => return (State::Current, file.display().to_string()),
        Ok(Held::Earlier) => "is an earlier klin skill, and klin setup replaces it".to_string(),
        Ok(Held::Other) => "differs from klin's skill".to_string(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            "is missing, and klin setup writes it".to_string()
        }
        Err(why) => format!("could not be read: {why}"),
    };
    (State::Conflict, format!("{} {wrong}", skill.display()))
}
