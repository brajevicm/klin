use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{Config, Error};
use crate::ratchet::{self, Finding, Gate, Values};

const SECTION: &str = "escapes";
const VERSION: &str = "1";
const PATTERNS: &[(&str, &str)] = &[
    ("unwrap", ".unwrap()"),
    ("expect", ".expect("),
    ("allow", "#[allow("),
];

#[derive(clap::Args)]
pub struct Args {
    /// The quality.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when the baseline is looser than the code — what CI runs
    #[arg(long)]
    strict: bool,
    /// Accept every escape site that exists today
    #[arg(long)]
    write_baseline: bool,
    /// Judge only these repo-relative files, against only their baseline entries
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

pub fn run(args: &Args, start: &Path) -> Result<u8, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let Some(section) = config.section(SECTION)?.as_object() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be an object",
            config.file.display()
        )));
    };
    let baseline_path = baseline_path(&config, section)?;
    let roots = roots(&config, section)?;
    let found = findings(&roots, config.root())?;
    let mut gate_config = section.clone();
    gate_config.remove("baseline");
    let measured = ratchet::provenance(SECTION, VERSION, &Value::Object(gate_config));

    if args.write_baseline {
        ratchet::write(&baseline_path, &found, &measured)?;
        println!("baseline written: {} escape site(s) accepted", found.len());
        return Ok(0);
    }

    let (entries, stored) = ratchet::read(&baseline_path)?;
    let (found, entries) = ratchet::restrict(found, entries, args.only.as_deref());
    let sites = found.len();
    let baseline_size = entries.len();
    let verdict = ratchet::judge(found, entries, &["count"], stored.as_ref(), Some(&measured));
    let gate = Gate {
        noun: "escape site(s)",
        over: "where the code opts out of a check",
        fix: "Fix what the escape hides: handle the error instead of unwrapping it, address the \
              lint instead of allowing it. Accepting a new escape into the baseline is a policy \
              decision for a person.",
        remedy: "detent escapes --write-baseline",
        show,
    };
    let ok_line = format!("OK: {sites} escape site(s) in the tree, all in the baseline");
    let mut out = String::new();
    let code = ratchet::report(
        &verdict,
        &gate,
        baseline_size,
        &ok_line,
        args.quiet,
        args.strict,
        &mut out,
    );
    print!("{out}");
    Ok(code)
}

fn baseline_path(config: &Config, section: &Values) -> Result<PathBuf, Error> {
    let named = section
        .get("baseline")
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, "baseline"))?;
    let name = named.rsplit(['/', '\\']).next().unwrap_or(named);
    if !name.contains("baseline") || !name.ends_with(".json") {
        return Err(Error(format!(
            "{}: \"{SECTION}\" names its baseline {named}, which the guard cannot recognise — \
             the file name must contain \"baseline\" and end with .json",
            config.file.display()
        )));
    }
    Ok(config.path(named))
}

fn roots(config: &Config, section: &Values) -> Result<Vec<PathBuf>, Error> {
    let Some(listed) = section.get("roots") else {
        return Ok(vec![config.root().to_path_buf()]);
    };
    let malformed = || config.malformed(SECTION, "roots", "a list of paths");
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
        .collect()
}

fn findings(roots: &[PathBuf], repo_root: &Path) -> Result<Vec<Finding>, Error> {
    let mut files = Vec::new();
    for root in roots {
        collect_rust_files(root, &mut files)?;
    }
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let bytes = std::fs::read(&file).map_err(|why| Error::unreadable(&file, why))?;
        let text = String::from_utf8_lossy(&bytes);
        let rel = file
            .strip_prefix(repo_root)
            .unwrap_or(&file)
            .display()
            .to_string();
        for (index, line) in text.lines().enumerate() {
            let Some((kind, count)) = PATTERNS
                .iter()
                .map(|(kind, pattern)| (*kind, line.matches(pattern).count()))
                .find(|(_, count)| *count > 0)
            else {
                continue;
            };
            let mut values = Values::new();
            values.insert("escape".into(), kind.into());
            values.insert("count".into(), count.into());
            out.push(Finding {
                file: rel.clone(),
                line: index as u64 + 1,
                text: line.trim().to_string(),
                values,
            });
        }
    }
    Ok(out)
}

fn collect_rust_files(root: &Path, into: &mut Vec<PathBuf>) -> Result<(), Error> {
    let listing = std::fs::read_dir(root).map_err(|why| Error::unreadable(root, why))?;
    for entry in listing {
        let entry = entry.map_err(|why| Error::unreadable(root, why))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                collect_rust_files(&path, into)?;
            }
        } else if name.ends_with(".rs") {
            into.push(path);
        }
    }
    Ok(())
}

fn show(values: &Values) -> String {
    let kind = values.get("escape").and_then(Value::as_str).unwrap_or("?");
    match values.get("count").and_then(Value::as_u64) {
        Some(count) if count > 1 => format!("{kind} x{count}"),
        _ => kind.to_string(),
    }
}
