use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::config::{Config, Error};
use crate::{complexity, doc_size, escapes};

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
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let configured: Vec<&str> = LADDER
        .iter()
        .filter(|(_, section)| config.section(section).is_ok())
        .map(|(name, _)| *name)
        .collect();
    let wanted = select(&args.gates, &configured, &config)?;
    if args.list {
        for name in &wanted {
            let _ = writeln!(out, "{name}");
        }
        return Ok(0);
    }
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
    let (mut failed, mut errored) = (0, 0);
    for name in &wanted {
        let (code, text) = one(name, &config.file, args.strict, start);
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
    let _ = writeln!(
        out,
        "detent: {} gate(s), {}",
        wanted.len(),
        summary(failed, errored)
    );
    Ok(if errored > 0 {
        2
    } else if failed > 0 {
        1
    } else {
        0
    })
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

fn one(name: &str, config: &Path, strict: bool, start: &Path) -> (u8, String) {
    let mut text = String::new();
    let at = Some(config.to_path_buf());
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
