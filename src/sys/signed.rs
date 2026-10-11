//! klin installs a newer release over itself only after its minisign signature checks out
//! against the public key built into this binary. The release workflow signs each archive with
//! the trusted comment `klin TAG TARGET`, so a signed archive of an older release, or of another
//! machine's build, never passes. Nothing is replaced before the check passes. Spec 11.9,
//! ADR 0070.

use std::fmt;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use minisign_verify::{PublicKey, Signature};

const DOWNLOAD: &str = "https://github.com/brajevicm/klin/releases/download/v";
const KEY: &str = include_str!("release.pub");

/// Why an install did not happen. A refusal holds for that release on every later try. Any
/// other failure, such as a download that did not finish, may pass on a later try.
pub enum Failure {
    Refused(String),
    Transient(String),
}

impl Failure {
    pub fn retry(&self) -> bool {
        matches!(self, Failure::Transient(_))
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Refused(why) | Failure::Transient(why) => out.write_str(why),
        }
    }
}

/// Download the archive of `tag` for this machine, check its signature, and put its `klin` in
/// place of `exe`. On an error, `exe` is as it was and the failure says why.
pub fn install(tag: &str, exe: &Path, scratch: &Path) -> Result<(), Failure> {
    let target = target().map_err(Failure::Refused)?;
    let archive = format!("klin-{target}.tar.xz");
    let url = format!("{DOWNLOAD}{tag}/{archive}");
    let work = Scratch(scratch.join(format!("update-{}", std::process::id())));
    let _ = fs::remove_dir_all(&work.0);
    fs::create_dir_all(&work.0)
        .map_err(|why| Failure::Transient(format!("{}: {why}", work.0.display())))?;
    let bytes = download(&url, &work.0.join(&archive)).map_err(Failure::Transient)?;
    let signature =
        download(&format!("{url}.minisig"), &work.0.join("minisig")).map_err(Failure::Transient)?;
    check(&bytes, &signature, &format!("klin {tag} {target}"), tag).map_err(Failure::Refused)?;
    let binary = unpack(&bytes, &work.0)?;
    replace(&binary, exe).map_err(Failure::Transient)
}

/// A path klin made for one install, removed when the install ends, whatever way it ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        let _ = fs::remove_file(&self.0);
    }
}

/// This machine's name in a release archive, for the four targets the release builds.
fn target() -> Result<&'static str, String> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "linux") => Ok("aarch64-unknown-linux-gnu"),
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu"),
        (arch, os) => Err(format!("no klin release is built for {arch} {os}.")),
    }
}

fn download(url: &str, to: &Path) -> Result<Vec<u8>, String> {
    let fetched = Command::new("curl")
        .args(["-fsSL", "--proto", "=https", "--max-time", "300", "-o"])
        .arg(to)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    match fetched {
        true => fs::read(to).map_err(|why| format!("{}: {why}", to.display())),
        false => Err(format!("{url} could not be downloaded.")),
    }
}

/// Whether `signature` is the release key's signature of `bytes`, and its trusted comment is
/// `names`: this release, for this machine.
fn check(bytes: &[u8], signature: &[u8], names: &str, tag: &str) -> Result<(), String> {
    let refused =
        |why: &str| format!("the signature of klin {tag} {why}, so nothing was installed.");
    let key = PublicKey::decode(&key()).map_err(|_| refused("could not be checked: the key"))?;
    let signature = std::str::from_utf8(signature)
        .ok()
        .and_then(|text| Signature::decode(text).ok())
        .ok_or_else(|| refused("is not a minisign signature"))?;
    key.verify(bytes, &signature, false)
        .map_err(|_| refused("does not match the release key"))?;
    match signature.trusted_comment() == names {
        true => Ok(()),
        false => Err(refused("names another release or another machine")),
    }
}

/// The release key. A debug build, which only the tests run, takes a test key from
/// `KLIN_RELEASE_KEY`, because the tests cannot sign with the release key.
fn key() -> String {
    #[cfg(debug_assertions)]
    if let Some(key) = std::env::var_os("KLIN_RELEASE_KEY") {
        return key.to_string_lossy().into_owned();
    }
    KEY.to_string()
}

/// Extract the checked `archive` bytes into `work`, through `tar`'s standard input so nothing
/// can change them on disk after the check, and return the path of the one `klin` they hold. A
/// `tar` that could not run may run later. A signed archive with no `klin` never changes.
fn unpack(archive: &[u8], work: &Path) -> Result<PathBuf, Failure> {
    let unpacked = work.join("unpacked");
    fs::create_dir_all(&unpacked)
        .map_err(|why| Failure::Transient(format!("{}: {why}", unpacked.display())))?;
    let extracted = Command::new("tar")
        .args(["-xJf", "-", "-C"])
        .arg(&unpacked)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok_and(|mut tar| {
            let fed = tar
                .stdin
                .take()
                .is_some_and(|mut input| input.write_all(archive).is_ok());
            tar.wait().is_ok_and(|status| status.success()) && fed
        });
    let found = fs::read_dir(&unpacked)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("klin"))
        .find(|binary| binary.is_file());
    match (extracted, found) {
        (true, Some(binary)) => Ok(binary),
        (true, None) => Err(Failure::Refused(
            "the release archive holds no klin, so nothing was installed.".to_string(),
        )),
        (false, _) => Err(Failure::Transient(
            "tar could not extract the release archive, so nothing was installed.".to_string(),
        )),
    }
}

/// Copy `binary` beside `exe` and rename it over `exe`, so `exe` is either the old klin or the
/// new one, never part of either.
fn replace(binary: &Path, exe: &Path) -> Result<(), String> {
    let beside = Scratch(exe.with_file_name(format!(".klin-new-{}", std::process::id())));
    let failed = |why: std::io::Error| format!("{}: {why}", exe.display());
    fs::copy(binary, &beside.0).map_err(failed)?;
    fs::set_permissions(&beside.0, fs::Permissions::from_mode(0o755)).map_err(failed)?;
    fs::rename(&beside.0, exe).map_err(failed)
}
