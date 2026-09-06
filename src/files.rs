use std::path::{Path, PathBuf};

use crate::config::{Config, Error};
use crate::ratchet::Values;

pub fn roots(
    config: &Config,
    section_name: &str,
    section: &Values,
    key: &str,
) -> Result<Option<Vec<PathBuf>>, Error> {
    let Some(listed) = section.get(key) else {
        return Ok(None);
    };
    let malformed = || config.malformed(section_name, key, "a list of paths");
    let Some(listed) = listed.as_array() else {
        return Err(malformed());
    };
    listed
        .iter()
        .map(|root| {
            root.as_str()
                .map(|name| config.path(name))
                .ok_or_else(malformed)
        })
        .collect::<Result<Vec<PathBuf>, Error>>()
        .map(Some)
}

pub fn under(roots: &[PathBuf], extensions: &[&str]) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for root in roots {
        walk(root, extensions, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn walk(root: &Path, extensions: &[&str], into: &mut Vec<PathBuf>) -> Result<(), Error> {
    let listing = std::fs::read_dir(root).map_err(|why| Error::unreadable(root, why))?;
    for entry in listing {
        let entry = entry.map_err(|why| Error::unreadable(root, why))?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|why| Error::unreadable(&path, why))?
            .is_symlink()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                walk(&path, extensions, into)?;
            }
        } else if extensions.iter().any(|extension| name.ends_with(extension)) {
            into.push(path);
        }
    }
    Ok(())
}

pub fn relative(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
