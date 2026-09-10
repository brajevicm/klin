//! `klin update` runs the `klin-update` program the installer puts beside this binary, which
//! installs the newest release over it. Spec 19.5.

use std::path::PathBuf;
use std::process::Command;

const INSTALL: &str = "curl --proto '=https' --tlsv1.2 -LsSf \
                       https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh";

pub fn run() -> u8 {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("klin-update")))
        .filter(|path| path.is_file());
    let program = beside.unwrap_or_else(|| PathBuf::from("klin-update"));
    match Command::new(&program).status() {
        Ok(status) => u8::try_from(status.code().unwrap_or(2)).unwrap_or(2),
        Err(_) => {
            eprintln!(
                "FAIL: klin-update was not found beside klin or on PATH. The installer puts it \
                 there: {INSTALL}. A plugin install updates with its plugin instead."
            );
            2
        }
    }
}
