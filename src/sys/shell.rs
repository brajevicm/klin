use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

const LIMIT: Duration = Duration::from_secs(300);

/// The process group of the command running now. Its lock is held from the spawn until the group
/// is recorded, so a signal that arrives in between waits for the group it must kill.
static RUNNING: Mutex<Option<Pid>> = Mutex::new(None);
/// The moment klin started, which `main` forces first, and which the deadline every command is
/// stopped at counts from. Spec 9.3.
pub static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);

/// One command a build or sarif entry names, in the shell, at the directory it runs in, in the
/// environment the hook itself was given, in a process group of its own. When the command ends,
/// reaches its limit or the deadline, or klin is told to stop, klin kills every process left in that group, and
/// its output is kept in files, so nothing it left behind can hold klin open. Spec 8.3, 9.3.
pub fn output(at: &Path, command: &str) -> Result<Output> {
    let (time, limit) = bound()?;
    let (stdout, stderr) = (tempfile::tempfile()?, tempfile::tempfile()?);
    let child = started(at, command, &stdout, &stderr)?;
    Ok(Output {
        status: waited(child, time, &limit)?,
        stdout: read(stdout)?,
        stderr: read(stderr)?,
    })
}

/// The command in a process group of its own, started only once klin watches for the signals
/// that would end it.
fn started(at: &Path, command: &str, stdout: &File, stderr: &File) -> Result<Child> {
    watched()?;
    let mut running = running();
    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(at)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .process_group(0)
        .spawn()?;
    *running = Some(Pid::from_child(&child));
    Ok(child)
}

fn running() -> MutexGuard<'static, Option<Pid>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How long the next command may run, and the limit that says so: its own, or what is left
/// before the deadline twice as long after klin started. A command with no time left never
/// starts. Spec 9.3.
fn bound() -> Result<(Duration, String)> {
    let limit = limit()?;
    let deadline = limit * 2;
    let left = deadline.saturating_sub(STARTED.elapsed());
    let named = format!(
        "the {} second deadline from klin's start",
        deadline.as_secs()
    );
    match left {
        left if left >= limit => Ok((limit, format!("the {} second limit", limit.as_secs()))),
        left if left.is_zero() => Err(Error::new(
            ErrorKind::TimedOut,
            format!("klin did not start it, because {named} passed"),
        )),
        left => Ok((left, named)),
    }
}

/// How long one command may run. `KLIN_COMMAND_LIMIT` shortens it for tests and can never
/// raise it. Spec 9.3.
fn limit() -> Result<Duration> {
    let Ok(pinned) = std::env::var("KLIN_COMMAND_LIMIT") else {
        return Ok(LIMIT);
    };
    match pinned.parse().map(Duration::from_secs) {
        Ok(limit) if !limit.is_zero() && limit <= LIMIT => Ok(limit),
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "KLIN_COMMAND_LIMIT is \"{pinned}\", which is not a count of seconds from 1 to {}",
                LIMIT.as_secs()
            ),
        )),
    }
}

/// The command's exit status once its group is killed and its shell reaped. The shell is
/// reaped last, so its group id names no other group while klin, or a signal, kills it.
fn waited(mut child: Child, time: Duration, named: &str) -> Result<ExitStatus> {
    let group = Pid::from_child(&child);
    let started = Instant::now();
    let ended = ended(group, started, time);
    {
        let mut running = running();
        let _ = kill_process_group(group, Signal::KILL);
        *running = None;
    }
    let status = child.wait();
    match ended? {
        true => status,
        false => Err(Error::new(
            ErrorKind::TimedOut,
            format!("klin stopped it at {named}"),
        )),
    }
}

/// Whether the shell exited before its time ran out. It is left unreaped.
fn ended(group: Pid, started: Instant, time: Duration) -> Result<bool> {
    let exited = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    loop {
        if waitid(WaitId::Pid(group), exited)?.is_some() {
            return Ok(true);
        }
        if started.elapsed() >= time {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A signal that ends klin kills the running command's group first, so a person's Ctrl-C or
/// the host's own timeout leaves no command running behind klin.
fn watched() -> Result<()> {
    static WATCHED: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    WATCHED
        .get_or_init(|| watcher().map_err(|why| why.to_string()))
        .clone()
        .map_err(|why| {
            Error::other(format!(
                "klin could not watch for the signals that end it: {why}"
            ))
        })
}

/// The thread that kills the running group and then ends klin as the signal would have. It
/// holds the group's lock until klin ends, so no other command starts in between.
fn watcher() -> Result<()> {
    let mut signals = Signals::new([SIGHUP, SIGINT, SIGTERM])?;
    std::thread::spawn(move || {
        for signal in signals.forever() {
            let running = running();
            if let Some(group) = *running {
                let _ = kill_process_group(group, Signal::KILL);
            }
            let _ = signal_hook::low_level::emulate_default_handler(signal);
            drop(running);
        }
    });
    Ok(())
}

fn read(mut file: File) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
