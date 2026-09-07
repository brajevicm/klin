use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;
use tree_sitter::{Node, Parser};

use crate::baseline::{self, Evaluator, Finding, Values};
use crate::config::{Config, Error, Flags};
use crate::files;

const SECTION: &str = "escapes";
const VERSION: &str = "1";

struct Language {
    names: &'static [&'static str],
    suffixes: &'static [&'static str],
    patterns: &'static [(&'static str, &'static str)],
}

const LANGUAGES: &[Language] = &[
    Language {
        names: &["go"],
        suffixes: &[".go"],
        patterns: &[
            ("nolint", r"//\s*nolint"),
            ("skipped test", r"\bt\.Skip(?:Now|f)?\("),
        ],
    },
    Language {
        names: &["java"],
        suffixes: &[".java"],
        patterns: &[
            ("suppress warnings", r"@SuppressWarnings\("),
            ("skipped test", r"@(?:Ignore|Disabled)\b"),
        ],
    },
    Language {
        names: &["kotlin"],
        suffixes: &[".kt", ".kts"],
        patterns: &[
            ("not-null assertion", r"!!"),
            ("suppress", r"@Suppress\("),
            ("skipped test", r"@(?:Ignore|Disabled)\b"),
        ],
    },
    Language {
        names: &["python"],
        suffixes: &[".py"],
        patterns: &[
            ("type ignore", r"#\s*type:\s*ignore"),
            ("noqa", r"#\s*noqa\b"),
            ("no cover", r"#\s*pragma:\s*no cover"),
            (
                "skipped test",
                r"pytest\.mark\.skip|pytest\.skip\(|unittest\.skip|@skip\b",
            ),
            ("bare except", r"^\s*except\s*:"),
        ],
    },
    Language {
        names: &["ruby"],
        suffixes: &[".rb"],
        patterns: &[
            ("rubocop:disable", r"rubocop:disable"),
            ("skipped test", r"\bskip\b|\bxit\b|\bpending\b"),
        ],
    },
    Language {
        names: &["rust"],
        suffixes: &[".rs"],
        patterns: &[
            ("unwrap", r"\.unwrap\(\)"),
            ("expect", r"\.expect\("),
            ("unsafe", r"\bunsafe\s*\{"),
            ("allow", r"#!?\[allow\("),
            ("todo", r"\b(?:todo|unimplemented)!\("),
            ("skipped test", r"#\[ignore\b"),
        ],
    },
    Language {
        names: &["shell"],
        suffixes: &[".sh", ".bash", ".zsh"],
        patterns: &[
            ("errors ignored", r"\|\|\s*true\b|^\s*set\s+\+e\b"),
            ("shellcheck disable", r"shellcheck\s+disable"),
        ],
    },
    Language {
        names: &["swift"],
        suffixes: &[".swift"],
        patterns: &[
            ("force try", r"\btry!"),
            ("force cast", r"\bas!"),
            ("force unwrap", r"[\w)\]]!(?:\.|\s*[,;)\]]|$)"),
            ("swiftlint:disable", r"swiftlint:disable"),
            ("unchecked Sendable", r"@unchecked\s+Sendable"),
            ("skipped test", r"\bXCTSkip|\bthrow\s+XCTSkip"),
        ],
    },
    Language {
        names: &["javascript", "typescript"],
        suffixes: &[".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"],
        patterns: &[
            ("any", r":\s*any\b|\bas\s+any\b|<any>"),
            ("ts-ignore", r"@ts-(?:ignore|expect-error|nocheck)"),
            ("eslint-disable", r"eslint-disable"),
            ("non-null assertion", r"[\w)\]]!\."),
            (
                "skipped test",
                r"\b(?:it|test|describe)\.(?:skip|only)\(|\bx(?:it|test|describe)\(",
            ),
        ],
    },
];

const EVERY_FILE: &str = "";
const PRELUDE: &[&str] = &[
    "attribute_item",
    "line_comment",
    "block_comment",
    "doc_comment",
];

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
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
    /// Print the built-in pattern sets and exit
    #[arg(long)]
    list_languages: bool,
    /// Judge only these repo-relative files, against only their baseline entries
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

struct Set {
    suffixes: Vec<String>,
    patterns: Vec<(String, Regex)>,
}

struct Spec {
    baseline: PathBuf,
    search: Search,
    roots: Vec<PathBuf>,
    measured: Values,
}

struct Search {
    sets: Vec<Set>,
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
    skip_rust_tests: bool,
}

struct Tally {
    line: u64,
    escape: String,
    count: u64,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if args.list_languages {
        list_languages(out);
        return Ok(0);
    }
    evaluate(&flags(args), args.write_baseline, start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, false, start, out)
}

fn evaluate(
    flags: &Flags,
    write_baseline: bool,
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    let spec = spec(&config)?;
    let (found, skipped) = findings(&spec.search, &spec.roots, config.root())?;
    let accepted = format!("baseline written: {} escape site(s) accepted", found.len());
    let sites = scoped(&found, flags.only.as_deref());
    let aside = match skipped {
        0 => String::new(),
        count => format!(" ({count} in inline Rust tests skipped)"),
    };
    evaluator(&spec).evaluate(
        found,
        flags,
        write_baseline,
        &format!("OK: {sites} escape site(s) in the tree, all in the baseline{aside}"),
        &accepted,
        out,
    )
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        quiet: args.quiet,
        strict: args.strict,
        only: args.only.clone(),
        records: None,
        with: None,
    }
}

fn scoped(found: &[Finding], only: Option<&[String]>) -> usize {
    match only {
        Some(only) => found
            .iter()
            .filter(|site| only.contains(&site.file))
            .count(),
        None => found.len(),
    }
}

fn spec(config: &Config) -> Result<Spec, Error> {
    let section = baseline::section(config, SECTION, VERSION)?;
    let values = &section.values;
    Ok(Spec {
        search: search(section.config, values)?,
        roots: files::roots(section.config, section.name, values, "roots")?
            .unwrap_or_else(|| vec![section.config.root().to_path_buf()]),
        baseline: section.baseline,
        measured: section.provenance,
    })
}

fn evaluator(spec: &Spec) -> Evaluator<'_> {
    Evaluator {
        path: &spec.baseline,
        provenance: &spec.measured,
        metrics: &["count"],
        unit: "escape site(s)",
        condition: "where the code opts out of a check",
        fix_advice: "Fix what the escape hides: handle the error instead of unwrapping it, \
                     address the lint instead of allowing it. Accepting a new escape into the \
                     baseline is a policy decision for a person.",
        tighten_command: "klin escapes --write-baseline",
        format_metrics: show,
        held_out: &[],
    }
}

fn language(name: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .find(|language| language.names.contains(&name))
}

fn list_languages(out: &mut String) {
    for language in LANGUAGES {
        let mut escapes: Vec<&str> = language.patterns.iter().map(|(name, _)| *name).collect();
        escapes.sort_unstable();
        let _ = writeln!(
            out,
            "{:<22} {}",
            language.names.join(", "),
            escapes.join(", ")
        );
    }
}

fn search(config: &Config, section: &Values) -> Result<Search, Error> {
    Ok(Search {
        sets: sets(config, section)?,
        skip_dirs: files::skip_dirs(config, SECTION, section)?,
        exclude: files::strings(config, SECTION, section, "exclude")?,
        skip_rust_tests: flag(config, section, "skip_rust_tests")?,
    })
}

fn sets(config: &Config, section: &Values) -> Result<Vec<Set>, Error> {
    let named = files::strings(config, SECTION, section, "languages")?;
    let mut sets = language_sets(config, &named)?;
    let project = project_patterns(config, section)?;
    if named.is_empty() && project.is_empty() {
        return Err(Error(format!(
            "{}: \"{SECTION}\" names no \"languages\" and no \"patterns\" — nothing to look for",
            config.file.display()
        )));
    }
    if !project.is_empty() {
        sets.push(project_set(config, &sets, project)?);
    }
    Ok(sets)
}

fn language_sets(config: &Config, named: &[String]) -> Result<Vec<Set>, Error> {
    named
        .iter()
        .map(|name| {
            let set = language(name).ok_or_else(|| unknown_language(config, name))?;
            Ok(Set {
                suffixes: set.suffixes.iter().map(|s| s.to_string()).collect(),
                patterns: compiled(
                    config,
                    set.patterns
                        .iter()
                        .map(|(name, regex)| (name.to_string(), regex.to_string())),
                )?,
            })
        })
        .collect()
}

fn project_set(
    config: &Config,
    sets: &[Set],
    project: Vec<(String, String)>,
) -> Result<Set, Error> {
    let mut suffixes: Vec<String> = sets.iter().flat_map(|set| set.suffixes.clone()).collect();
    suffixes.sort();
    suffixes.dedup();
    if suffixes.is_empty() {
        suffixes.push(EVERY_FILE.to_string());
    }
    Ok(Set {
        suffixes,
        patterns: compiled(config, project.into_iter())?,
    })
}

fn unknown_language(config: &Config, name: &str) -> Error {
    let known: Vec<&str> = LANGUAGES
        .iter()
        .flat_map(|language| language.names.iter().copied())
        .collect();
    Error(format!(
        "{}: \"{SECTION}\" names no built-in escape patterns for \"{name}\" — one of: {}",
        config.file.display(),
        known.join(", ")
    ))
}

fn compiled(
    config: &Config,
    patterns: impl Iterator<Item = (String, String)>,
) -> Result<Vec<(String, Regex)>, Error> {
    patterns
        .map(|(name, regex)| {
            Regex::new(&format!("(?m){regex}"))
                .map(|compiled| (name.clone(), compiled))
                .map_err(|why| {
                    Error(format!(
                        "{}: \"{SECTION}\" pattern \"{name}\" is not a regular expression: {why}",
                        config.file.display()
                    ))
                })
        })
        .collect()
}

fn flag(config: &Config, section: &Values, key: &str) -> Result<bool, Error> {
    match section.get(key) {
        None => Ok(true),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| config.malformed(SECTION, key, "true or false")),
    }
}

fn project_patterns(config: &Config, section: &Values) -> Result<Vec<(String, String)>, Error> {
    let Some(listed) = section.get("patterns") else {
        return Ok(Vec::new());
    };
    let malformed = || config.malformed(SECTION, "patterns", "an object of name to regex");
    listed
        .as_object()
        .ok_or_else(malformed)?
        .iter()
        .map(|(name, regex)| {
            regex
                .as_str()
                .map(|regex| (name.clone(), regex.to_string()))
                .ok_or_else(malformed)
        })
        .collect()
}

fn findings(
    search: &Search,
    roots: &[PathBuf],
    repo_root: &Path,
) -> Result<(Vec<Finding>, u64), Error> {
    let mut seen: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut ranges: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    let mut skipped = 0;
    for set in &search.sets {
        let suffixes: Vec<&str> = set.suffixes.iter().map(String::as_str).collect();
        let wanted = files::Wanted {
            extensions: &suffixes,
            skip_dirs: &search.skip_dirs,
            exclude: &search.exclude,
            exclude_except: &[],
            skip_hidden: false,
        };
        for file in files::under(roots, &wanted)? {
            let bytes = std::fs::read(&file).map_err(|why| Error::unreadable(&file, why))?;
            let text = String::from_utf8_lossy(&bytes).to_string();
            let rel = files::relative(&file, repo_root);
            let tests = cached_test_ranges(search, &rel, &text, &mut ranges);
            skipped += tally(set, &rel, &text, &tests, &mut seen);
        }
    }
    Ok((collected(seen), skipped))
}

fn cached_test_ranges(
    search: &Search,
    rel: &str,
    text: &str,
    ranges: &mut BTreeMap<String, Vec<(u64, u64)>>,
) -> Vec<(u64, u64)> {
    if !(search.skip_rust_tests && rel.ends_with(".rs")) {
        return Vec::new();
    }
    ranges
        .entry(rel.to_string())
        .or_insert_with(|| rust_test_ranges(text))
        .clone()
}

fn tally(
    set: &Set,
    rel: &str,
    text: &str,
    tests: &[(u64, u64)],
    seen: &mut BTreeMap<(String, String), Tally>,
) -> u64 {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut skipped = 0;
    for (escape, regex) in &set.patterns {
        for found in regex.find_iter(text) {
            let line = text[..found.start()].matches('\n').count() as u64 + 1;
            if tests.iter().any(|(from, to)| *from <= line && line <= *to) {
                skipped += 1;
                continue;
            }
            let body = lines.get(line as usize - 1).unwrap_or(&"").trim();
            record(seen, rel, line, body, escape);
        }
    }
    skipped
}

fn collected(seen: BTreeMap<(String, String), Tally>) -> Vec<Finding> {
    let mut out: Vec<Finding> = seen
        .into_iter()
        .map(|((file, text), tally)| {
            let mut values = Values::new();
            values.insert("escape".into(), tally.escape.into());
            values.insert("count".into(), tally.count.into());
            Finding {
                file,
                line: tally.line,
                text,
                values,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn record(
    seen: &mut BTreeMap<(String, String), Tally>,
    file: &str,
    line: u64,
    text: &str,
    escape: &str,
) {
    seen.entry((file.to_string(), text.to_string()))
        .and_modify(|tally| tally.count += 1)
        .or_insert(Tally {
            line,
            escape: escape.to_string(),
            count: 1,
        });
}

fn rust_test_ranges(source: &str) -> Vec<(u64, u64)> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    test_ranges(tree.root_node(), source.as_bytes(), &mut out);
    out
}

fn test_ranges(node: Node, source: &[u8], out: &mut Vec<(u64, u64)>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" && is_cfg_test(child, source) {
            out.push((
                child.start_position().row as u64 + 1,
                item_after(child).end_position().row as u64 + 1,
            ));
            continue;
        }
        test_ranges(child, source, out);
    }
}

fn item_after(attribute: Node) -> Node {
    let mut node = attribute;
    while let Some(next) = node.next_named_sibling() {
        node = next;
        if !PRELUDE.contains(&node.kind()) {
            break;
        }
    }
    node
}

fn is_cfg_test(node: Node, source: &[u8]) -> bool {
    node.utf8_text(source)
        .is_ok_and(|text| text.split_whitespace().collect::<String>() == "#[cfg(test)]")
}

fn show(values: &Values) -> String {
    let escape = values.get("escape").and_then(Value::as_str).unwrap_or("?");
    match values.get("count").and_then(Value::as_u64) {
        Some(count) if count > 1 => format!("{escape} x{count}"),
        _ => escape.to_string(),
    }
}
