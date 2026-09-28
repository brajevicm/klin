use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(300);

/// One command a build or sarif entry names, in the shell, at the directory it runs in, in the
/// environment the hook itself was given. A command still running at the limit is stopped with
/// every process in its group, and its output is kept in files, so a process it left behind
/// cannot hold klin open. Spec 8.3, 9.3.
pub fn output(at: &Path, command: &str) -> Result<Output> {
    let limit = limit()?;
    let (stdout, stderr) = (tempfile::tempfile()?, tempfile::tempfile()?);
    let child = started(at, command, &stdout, &stderr)?;
    Ok(Output {
        status: waited(child, limit)?,
        stdout: read(stdout)?,
        stderr: read(stderr)?,
    })
}

/// The command in a process group of its own, so the limit stops every process it started.
fn started(at: &Path, command: &str, stdout: &File, stderr: &File) -> Result<Child> {
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

/// How long one command may run. `KLIN_COMMAND_LIMIT` overrides it for tests. Spec 9.3.
fn limit() -> Result<Duration> {
    let Ok(pinned) = std::env::var("KLIN_COMMAND_LIMIT") else {
        return Ok(LIMIT);
    };
    pinned.parse().map(Duration::from_secs).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("KLIN_COMMAND_LIMIT is \"{pinned}\", which is not a count of seconds"),
        )
    })
}

fn waited(mut child: Child, limit: Duration) -> Result<ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= limit {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    stop(&mut child);
    Err(Error::new(
        ErrorKind::TimedOut,
        format!("klin stopped it at the {} second limit", limit.as_secs()),
    ))
}

fn stop(child: &mut Child) {
    let _ = Command::new("kill")
        .args(["-s", "KILL", "--", &format!("-{}", child.id())])
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

fn read(mut file: File) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
