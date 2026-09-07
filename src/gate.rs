use std::fmt::Write;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{Config, Error, Flags};
use crate::{changed, complexity, doc_size, escapes};

const BUILD_BLOCKED: &str = ".detent-build-blocked";

struct Gate {
    name: &'static str,
    section: &'static str,
    run: fn(&Flags, &Path, &mut String) -> Result<u8, Error>,
    holds_baseline: bool,
}

const GATES: &[Gate] = &[
    Gate {
        name: "doc-size",
        section: "doc_size",
        run: doc_size::gate,
        holds_baseline: false,
    },
    Gate {
        name: "escapes",
        section: "escapes",
        run: escapes::gate,
        holds_baseline: true,
    },
    Gate {
        name: "complexity",
        section: "complexity",
        run: complexity::gate,
        holds_baseline: true,
    },
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
        return outcome.map(|(failed, errored)| code(failed, errored));
    }
    let (failed, errored) = match outcome {
        Ok(tally) => tally,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            (0, 1)
        }
    };
    Ok(hook(
        failed,
        errored,
        &std::mem::take(out),
        &root(args, start),
    ))
}

fn judge(args: &Args, start: &Path, out: &mut String) -> Result<(usize, usize), Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let configured = configured(&config)?;
    let wanted = select(&args.gates, &configured, &config)?;
    if wanted.is_empty() {
        return Err(Error(format!(
            "{} configures no gate — name at least one of: {}",
            config.file.display(),
            names(GATES.iter())
        )));
    }
    if args.list {
        for gate in &wanted {
            let _ = writeln!(out, "{}", gate.name);
        }
        return Ok((0, 0));
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
    Ok((failed, errored))
}

fn hook(failed: usize, errored: usize, report: &str, root: &Path) -> u8 {
    let unspent = build_blocked(root);
    if failed == 0 && errored == 0 {
        return 0;
    }
    let Some(event) = event() else {
        eprint!("{report}");
        return 1;
    };
    let again = !unspent
        && event
            .get("stop_hook_active")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    eprintln!("detent: {}{tail}", lead(failed, errored));
    eprint!("{report}");
    if !again {
        return 2;
    }
    eprintln!("detent: not blocking a second time; the failure stands and CI will refuse it.");
    0
}

fn lead(failed: usize, errored: usize) -> &'static str {
    match (failed > 0, errored > 0) {
        (true, true) => "a quality gate failed, and another could not run",
        (true, false) => "a quality gate failed",
        _ => "could not run a quality gate",
    }
}

fn root(args: &Args, start: &Path) -> PathBuf {
    Config::load(args.config.as_deref(), start)
        .map(|config| config.root().to_path_buf())
        .unwrap_or_else(|_| start.to_path_buf())
}

fn build_blocked(root: &Path) -> bool {
    std::fs::remove_file(root.join(BUILD_BLOCKED)).is_ok()
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

fn names<'a>(gates: impl Iterator<Item = &'a Gate>) -> String {
    gates
        .map(|gate| gate.name)
        .collect::<Vec<&str>>()
        .join(", ")
}

fn configured(config: &Config) -> Result<Vec<&'static Gate>, Error> {
    if let Some(gate) = GATES
        .iter()
        .find(|gate| gate.name != gate.section && config.section(gate.name).is_ok())
    {
        return Err(Error(format!(
            "{}: \"{}\" is what the command is called — the section it reads is \"{}\"",
            config.file.display(),
            gate.name,
            gate.section
        )));
    }
    Ok(GATES
        .iter()
        .filter(|gate| config.section(gate.section).is_ok())
        .collect())
}

fn each(
    wanted: &[&'static Gate],
    config: &Path,
    strict: bool,
    start: &Path,
    scope: Option<&[String]>,
    out: &mut String,
) -> (usize, usize) {
    let (mut failed, mut errored) = (0, 0);
    for gate in wanted {
        let (code, text) = one(gate, config, strict, start, scope);
        match code {
            0 => (),
            1 => failed += 1,
            _ => errored += 1,
        }
        let _ = writeln!(out, "  {}  {}", status(code), gate.name);
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
    configured: &[&'static Gate],
    config: &Config,
) -> Result<Vec<&'static Gate>, Error> {
    if let Some(name) = named
        .iter()
        .find(|name| !configured.iter().any(|gate| gate.name == name.as_str()))
    {
        return Err(Error(format!(
            "no gate named {name} — {} configures: {}",
            config.file.display(),
            match configured.is_empty() {
                true => "nothing".to_string(),
                false => names(configured.iter().copied()),
            }
        )));
    }
    Ok(configured
        .iter()
        .copied()
        .filter(|gate| named.is_empty() || named.iter().any(|wanted| wanted == gate.name))
        .collect())
}

fn one(
    gate: &Gate,
    config: &Path,
    strict: bool,
    start: &Path,
    scope: Option<&[String]>,
) -> (u8, String) {
    let mut text = String::new();
    let flags = Flags {
        config: Some(config.to_path_buf()),
        quiet: true,
        strict: strict && gate.holds_baseline,
        only: scope
            .filter(|_| gate.holds_baseline)
            .map(<[String]>::to_vec),
    };
    match (gate.run)(&flags, start, &mut text) {
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
