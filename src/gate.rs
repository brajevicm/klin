use std::fmt::Write;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{Config, Error};
use crate::{changed, complexity, doc_size, escapes};

const LADDER: &[(&str, &str)] = &[
    ("doc-size", "doc_size"),
    ("escapes", "escapes"),
    ("complexity", "complexity"),
];

#[derive(clap::Args)]
pub struct Args {
    /// The quality.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fail when a baseline is looser than the code — what CI runs
    #[arg(long)]
    strict: bool,
    /// Run only this gate (repeatable)
    #[arg(long = "gate", value_name = "NAME")]
    gates: Vec<String>,
    /// Print the configured gates and exit
    #[arg(long)]
    list: bool,
    /// Judge only the files changed against the base — the fast loop; CI runs the full pass
    #[arg(long)]
    changed: bool,
    /// Agent Stop hook mode: the failures to stderr, exit 2 to block the first stop
    #[arg(long)]
    hook: bool,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let outcome = judge(args, start, out);
    if !args.hook {
        return outcome;
    }
    let code = match outcome {
        Ok(code) => code,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            2
        }
    };
    Ok(hook(code, &std::mem::take(out)))
}

fn judge(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let configured = configured(&config)?;
    let wanted = select(&args.gates, &configured, &config)?;
    if wanted.is_empty() {
        return Err(Error(format!(
            "{} configures no gate — name at least one of: {}",
            config.file.display(),
            LADDER
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<&str>>()
                .join(", ")
        )));
    }
    if args.list {
        for name in &wanted {
            let _ = writeln!(out, "{name}");
        }
        return Ok(0);
    }
    let scope = scope(args, &config, out)?;
    let (failed, errored) = each(
        &wanted,
        &config.file,
        args.strict,
        start,
        scope.as_deref(),
        out,
    );
    let _ = writeln!(
        out,
        "detent: {} gate(s), {}",
        wanted.len(),
        summary(failed, errored)
    );
    Ok(code(failed, errored))
}

fn hook(code: u8, report: &str) -> u8 {
    if code == 0 {
        return 0;
    }
    let Some(event) = event() else {
        eprint!("{report}");
        return 1;
    };
    let again = event
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    eprintln!("detent: a quality gate failed{tail}");
    eprint!("{report}");
    if !again {
        return 2;
    }
    eprintln!("detent: not blocking a second time; the failure stands and CI will refuse it.");
    0
}

fn event() -> Option<Value> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

fn scope(args: &Args, config: &Config, out: &mut String) -> Result<Option<Vec<String>>, Error> {
    if !args.changed {
        return Ok(None);
    }
    let files = changed::files(config.root())?;
    let _ = writeln!(
        out,
        "  changed: {} file(s) against the base — the scoped gates judge those; \
         CI judges everything",
        files.len()
    );
    Ok(Some(files))
}

fn configured(config: &Config) -> Result<Vec<&'static str>, Error> {
    if let Some((name, section)) = LADDER
        .iter()
        .find(|(name, section)| name != section && config.section(name).is_ok())
    {
        return Err(Error(format!(
            "{}: \"{name}\" is what the command is called — the section it reads is \"{section}\"",
            config.file.display()
        )));
    }
    Ok(LADDER
        .iter()
        .filter(|(_, section)| config.section(section).is_ok())
        .map(|(name, _)| *name)
        .collect())
}

fn each(
    wanted: &[&str],
    config: &Path,
    strict: bool,
    start: &Path,
    scope: Option<&[String]>,
    out: &mut String,
) -> (usize, usize) {
    let (mut failed, mut errored) = (0, 0);
    for name in wanted {
        let (code, text) = one(name, config, strict, start, scope);
        match code {
            0 => (),
            1 => failed += 1,
            _ => errored += 1,
        }
        let _ = writeln!(out, "  {}  {name}", status(code));
        for line in text.lines() {
            let _ = writeln!(out, "        {line}");
        }
    }
    (failed, errored)
}

fn code(failed: usize, errored: usize) -> u8 {
    if errored > 0 {
        2
    } else if failed > 0 {
        1
    } else {
        0
    }
}

fn select(
    named: &[String],
    configured: &[&'static str],
    config: &Config,
) -> Result<Vec<&'static str>, Error> {
    if let Some(name) = named
        .iter()
        .find(|name| !configured.contains(&name.as_str()))
    {
        return Err(Error(format!(
            "no gate named {name} — {} configures: {}",
            config.file.display(),
            match configured.is_empty() {
                true => "nothing".to_string(),
                false => configured.join(", "),
            }
        )));
    }
    Ok(configured
        .iter()
        .copied()
        .filter(|name| named.is_empty() || named.iter().any(|wanted| wanted == name))
        .collect())
}

fn one(
    name: &str,
    config: &Path,
    strict: bool,
    start: &Path,
    scope: Option<&[String]>,
) -> (u8, String) {
    let mut text = String::new();
    let at = Some(config.to_path_buf());
    let only = scope.map(|files| files.to_vec());
    let outcome = match name {
        "doc-size" => doc_size::run(
            &doc_size::Args {
                config: at,
                quiet: true,
                ..Default::default()
            },
            start,
            &mut text,
        ),
        "escapes" => escapes::run(
            &escapes::Args {
                config: at,
                quiet: true,
                strict,
                only,
                ..Default::default()
            },
            start,
            &mut text,
        ),
        "complexity" => complexity::run(
            &complexity::Args {
                config: at,
                quiet: true,
                strict,
                only,
                ..Default::default()
            },
            start,
            &mut text,
        ),
        _ => Err(Error(format!("the ladder names {name}, nothing runs it"))),
    };
    match outcome {
        Ok(code) => (code, text),
        Err(problem) => (2, text + &format!("FAIL: {problem}")),
    }
}

fn status(code: u8) -> &'static str {
    match code {
        0 => "ok  ",
        1 => "FAIL",
        _ => "ERR ",
    }
}

fn summary(failed: usize, errored: usize) -> String {
    let mut parts = Vec::new();
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    if errored > 0 {
        let plural = if errored == 1 { "" } else { "s" };
        parts.push(format!("{errored} tool error{plural}"));
    }
    match parts.is_empty() {
        true => "all passed.".to_string(),
        false => parts.join(", ") + ".",
    }
}
