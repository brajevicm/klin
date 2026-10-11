//! klin installs a newer release over itself only after its minisign signature checks out
//! against the public key built into this binary. The release workflow signs each archive with
//! the trusted comment `klin TAG`, so a signed archive of an older release never passes as a
//! newer one. Nothing is replaced before the check passes. Spec 11.9, ADR 0070.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use minisign_verify::{PublicKey, Signature};

const DOWNLOAD: &str = "https://github.com/brajevicm/klin/releases/download/v";
const KEY: &str = include_str!("release.pub");

/// Download the archive of `tag` for this machine, check its signature, and put its `klin` in
/// place of `exe`. On an error, `exe` is as it was and the message says why.
pub fn install(tag: &str, exe: &Path, scratch: &Path) -> Result<(), String> {
    let archive = format!("klin-{}.tar.xz", target()?);
    let url = format!("{DOWNLOAD}{tag}/{archive}");
    let work = scratch.join(format!("update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|why| format!("{}: {why}", work.display()))?;
    let installed = download(&url, &work.join(&archive))
        .and_then(|bytes| {
            let signature = download(&format!("{url}.minisig"), &work.join("minisig"))?;
            check(&bytes, &signature, tag)
        })
        .and_then(|()| unpack(&work.join(&archive), &work))
        .and_then(|binary| replace(&binary, exe));
    let _ = fs::remove_dir_all(&work);
    installed
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

/// Whether `signature` is the release key's signature of `bytes`, made for `tag`.
fn check(bytes: &[u8], signature: &[u8], tag: &str) -> Result<(), String> {
    let refused =
        |why: &str| format!("the signature of klin {tag} {why}, so nothing was installed.");
    let key = PublicKey::decode(&key()).map_err(|_| refused("could not be checked: the key"))?;
    let signature = std::str::from_utf8(signature)
        .ok()
        .and_then(|text| Signature::decode(text).ok())
        .ok_or_else(|| refused("is not a minisign signature"))?;
    key.verify(bytes, &signature, false)
        .map_err(|_| refused("does not match the release key"))?;
    match signature.trusted_comment() == format!("klin {tag}") {
        true => Ok(()),
        false => Err(refused("names another release")),
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

/// Extract `archive` into `work` and return the path of the one `klin` it holds.
fn unpack(archive: &Path, work: &Path) -> Result<PathBuf, String> {
    let unpacked = work.join("unpacked");
    fs::create_dir_all(&unpacked).map_err(|why| format!("{}: {why}", unpacked.display()))?;
    let extracted = Command::new("tar")
        .arg("-xJf")
        .arg(archive)
        .arg("-C")
        .arg(&unpacked)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let found = fs::read_dir(&unpacked)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("klin"))
        .find(|binary| binary.is_file());
    match (extracted, found) {
        (true, Some(binary)) => Ok(binary),
        _ => Err(format!("{} holds no klin.", archive.display())),
    }
}

/// Copy `binary` beside `exe` and rename it over `exe`, so `exe` is either the old klin or the
/// new one, never part of either.
fn replace(binary: &Path, exe: &Path) -> Result<(), String> {
    let beside = exe.with_file_name(".klin-update.new");
    let failed = |why: std::io::Error| format!("{}: {why}", exe.display());
    fs::copy(binary, &beside).map_err(failed)?;
    fs::set_permissions(&beside, fs::Permissions::from_mode(0o755)).map_err(failed)?;
    fs::rename(&beside, exe).map_err(|why| {
        let _ = fs::remove_file(&beside);
        failed(why)
    })
}
