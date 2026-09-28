use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

const LIMIT: Duration = Duration::from_secs(300);

/// The process group of the command running now, or 0 when none runs.
static RUNNING: AtomicI32 = AtomicI32::new(0);
/// The milliseconds the commands of this run have taken so far.
static SPENT: AtomicU64 = AtomicU64::new(0);

/// One command a build or sarif entry names, in the shell, at the directory it runs in, in the
/// environment the hook itself was given, in a process group of its own. When the command ends,
/// reaches its limit, or klin is told to stop, klin kills every process left in that group, and
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
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(at)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .process_group(0)
        .spawn()
}

/// How long the next command may run, and the limit that says so: its own, or what is left of
/// the twice as long the commands of one run share. Spec 9.3.
fn bound() -> Result<(Duration, String)> {
    let limit = limit()?;
    let shared = limit * 2;
    let left = shared.saturating_sub(Duration::from_millis(SPENT.load(Ordering::SeqCst)));
    Ok(match left < limit {
        false => (limit, format!("the {} second limit", limit.as_secs())),
        true => (
            left,
            format!(
                "the {} second limit that the commands of one run share",
                shared.as_secs()
            ),
        ),
    })
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
/// reaped last, so its group id names no other group while klin kills it.
fn waited(mut child: Child, time: Duration, limit: &str) -> Result<ExitStatus> {
    let group = Pid::from_child(&child);
    RUNNING.store(group.as_raw_nonzero().get(), Ordering::SeqCst);
    let started = Instant::now();
    let ended = ended(group, started, time);
    let _ = kill_process_group(group, Signal::KILL);
    RUNNING.store(0, Ordering::SeqCst);
    let status = child.wait();
    let taken = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    SPENT.fetch_add(taken, Ordering::SeqCst);
    match ended? {
        true => status,
        false => Err(Error::new(
            ErrorKind::TimedOut,
            format!("klin stopped it at {limit}"),
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
    static WATCHED: OnceLock<bool> = OnceLock::new();
    match WATCHED.get_or_init(|| watch().is_ok()) {
        true => Ok(()),
        false => Err(Error::other(
            "klin could not watch for the signals that end it",
        )),
    }
}

fn watch() -> Result<()> {
    let mut signals = Signals::new([SIGHUP, SIGINT, SIGTERM])?;
    std::thread::spawn(move || {
        for signal in signals.forever() {
            if let Some(group) = Pid::from_raw(RUNNING.load(Ordering::SeqCst)) {
                let _ = kill_process_group(group, Signal::KILL);
            }
            let _ = signal_hook::low_level::emulate_default_handler(signal);
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
