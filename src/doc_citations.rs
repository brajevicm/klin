use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{Config, Error, Flags};
use crate::files;
use crate::ratchet::Values;

const SECTION: &str = "doc_citations";
const DEFAULT_EXTENSIONS: &[&str] = &[
    ".py", ".ts", ".tsx", ".js", ".jsx", ".swift", ".rs", ".go", ".kt", ".java", ".rb", ".sh",
    ".md", ".json", ".yml", ".yaml", ".toml",
];
const EVERY_FILE: &str = "";
const REMEDY: &str = "Point the citation at where the file is now (a bare filename resolves \
    when exactly one file under the roots has that name), or delete the sentence that cites it.";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Judge this one document instead of the config's list
    #[arg(long)]
    file: Option<PathBuf>,
    /// Where --file's citations may resolve (repeatable; standalone, needs no config entry)
    #[arg(long = "root")]
    roots: Vec<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
}

struct Document {
    path: PathBuf,
    roots: Vec<PathBuf>,
    written: Vec<String>,
    extensions: Vec<String>,
    name: String,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if let Some(named) = &args.file
        && !named.is_file()
    {
        return Err(Error(format!("no such file: {}", named.display())));
    }
    evaluate(&flags(args), args.file.as_deref(), &args.roots, start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, None, &[], start, out)
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: SECTION.to_string(),
        prior: None,
        quiet: args.quiet,
        strict: false,
        only: None,
        records: None,
        with: None,
    }
}

fn evaluate(
    flags: &Flags,
    named: Option<&Path>,
    roots: &[PathBuf],
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let mut failed = 0;
    let mut indexes: HashMap<Vec<PathBuf>, BTreeMap<String, Vec<String>>> = HashMap::new();
    for document in documents(flags, named, roots, start)? {
        failed += usize::from(judge(&document, &mut indexes, flags, out)?);
    }
    Ok(if failed > 0 { 1 } else { 0 })
}

/// `indexes` is built once per distinct set of roots in a run, not once per document — several
/// entries sharing `"roots": ["."]` walk the tree once, not once each.
fn judge(
    document: &Document,
    indexes: &mut HashMap<Vec<PathBuf>, BTreeMap<String, Vec<String>>>,
    flags: &Flags,
    out: &mut String,
) -> Result<bool, Error> {
    if !document.path.is_file() {
        return Err(Error(format!("no such file: {}", document.path.display())));
    }
    for root in &document.roots {
        if !root.is_dir() {
            return Err(Error(format!("no such directory: {}", root.display())));
        }
    }
    let text =
        std::fs::read(&document.path).map_err(|why| Error::unreadable(&document.path, why))?;
    let text = String::from_utf8_lossy(&text);
    let cited = citations(&text, &document.extensions);
    let index = match indexes.get(&document.roots) {
        Some(index) => index,
        None => {
            indexes.insert(document.roots.clone(), basenames_under(&document.roots)?);
            &indexes[&document.roots]
        }
    };
    let missing: Vec<(String, u64, String)> = cited
        .iter()
        .filter_map(|(path, line)| {
            resolves(path, &document.roots, index).map(|why| (path.clone(), *line, why))
        })
        .collect();
    report(document, cited.len(), &missing, flags, out)
}

fn report(
    document: &Document,
    cited: usize,
    missing: &[(String, u64, String)],
    flags: &Flags,
    out: &mut String,
) -> Result<bool, Error> {
    let name = &document.name;
    if missing.is_empty() {
        if !flags.quiet {
            let _ = writeln!(out, "OK: {name} — all {cited} cited path(s) resolve");
        }
        return Ok(false);
    }
    let roots = document.written.join(", ");
    let _ = writeln!(
        out,
        "FAIL: {name} cites {} path(s) that resolve nowhere under {roots}:",
        missing.len()
    );
    for (path, line, why) in missing.iter().take(20) {
        let _ = writeln!(out, "  {name}:{line}  `{path}` — {why}");
    }
    let _ = writeln!(out, "{REMEDY}");
    flags.record(|records| {
        for (path, line, why) in missing {
            records
                .findings
                .push(Value::Object(site(name, *line, path, why)));
        }
    });
    Ok(true)
}

fn site(name: &str, line: u64, path: &str, why: &str) -> Values {
    let mut values = Values::new();
    values.insert("path".into(), path.into());
    values.insert("resolution".into(), why.into());
    let mut out = Values::new();
    out.insert("outcome".into(), "new".into());
    out.insert("file".into(), name.into());
    out.insert("line".into(), line.into());
    out.insert("values".into(), Value::Object(values));
    out
}

/// Every backticked span on each line that looks like a path with one of `extensions` — no
/// spaces, no `*`, and a `/` or a `.` — with a trailing `:line` suffix stripped.
fn citations(text: &str, extensions: &[String]) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let ticks: Vec<usize> = line.match_indices('`').map(|(at, _)| at).collect();
        for pair in ticks.as_chunks::<2>().0 {
            if let Some(candidate) = candidate(&line[pair[0] + 1..pair[1]], extensions) {
                out.push((candidate, number as u64 + 1));
            }
        }
    }
    out
}

fn candidate(span: &str, extensions: &[String]) -> Option<String> {
    let candidate = span.trim().split(':').next().unwrap_or("");
    if candidate.contains(' ') || candidate.contains('*') {
        return None;
    }
    if !extensions
        .iter()
        .any(|extension| candidate.ends_with(extension.as_str()))
    {
        return None;
    }
    (candidate.contains('/') || candidate.contains('.')).then(|| candidate.to_string())
}

/// {basename: [repo-relative paths]} for every file under the roots, so a bare filename cited
/// without its directory resolves through this, when it is unique.
fn basenames_under(roots: &[PathBuf]) -> Result<BTreeMap<String, Vec<String>>, Error> {
    let wanted = files::Wanted {
        extensions: &[EVERY_FILE],
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: false,
    };
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for file in files::under(std::slice::from_ref(root), &wanted)? {
            let Some(name) = file
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
            else {
                continue;
            };
            index
                .entry(name)
                .or_default()
                .push(files::relative(&file, root));
        }
    }
    for found in index.values_mut() {
        found.sort();
        found.dedup();
    }
    Ok(index)
}

/// None when `path` resolves — under a root as written, or as a bare filename found exactly
/// once — else the reason it does not.
fn resolves(
    path: &str,
    roots: &[PathBuf],
    index: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    if roots.iter().any(|root| root.join(path).is_file()) {
        return None;
    }
    if path.contains('/') {
        return Some("not under the roots".to_string());
    }
    match index.get(path).map(Vec::as_slice) {
        Some([_]) => None,
        Some(found) if !found.is_empty() => Some(format!(
            "ambiguous — cite one: {}",
            found[..found.len().min(4)].join(", ")
        )),
        _ => Some("no file of that name under the roots".to_string()),
    }
}

fn documents(
    flags: &Flags,
    named: Option<&Path>,
    roots: &[PathBuf],
    start: &Path,
) -> Result<Vec<Document>, Error> {
    if let Some(named) = named
        && !roots.is_empty()
    {
        return Ok(vec![Document {
            path: named.to_path_buf(),
            roots: roots.to_vec(),
            written: roots
                .iter()
                .map(|root| root.display().to_string())
                .collect(),
            extensions: default_extensions(),
            name: named.display().to_string(),
        }]);
    }
    let config = Config::open(flags, start)?;
    let listed = listed_documents(&config)?;
    let Some(named) = named else {
        return Ok(listed);
    };
    let wanted = identity(named);
    for document in listed {
        if identity(&document.path) == wanted {
            return Ok(vec![document]);
        }
    }
    Err(Error(format!(
        "{}: no \"{SECTION}\" entry for {} — pass --root DIR",
        config.file.display(),
        named.display()
    )))
}

fn listed_documents(config: &Config) -> Result<Vec<Document>, Error> {
    let Some(entries) = config.section(SECTION)?.as_array() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be a list of {{\"file\", \"roots\"}} entries",
            config.file.display()
        )));
    };
    entries
        .iter()
        .map(|entry| document(config, entry))
        .collect()
}

fn document(config: &Config, entry: &Value) -> Result<Document, Error> {
    let values = entry
        .as_object()
        .ok_or_else(|| config.malformed(SECTION, "file", "an object"))?;
    let name = values
        .get("file")
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, "file"))?;
    let roots = files::roots(config, SECTION, values, "roots")?
        .unwrap_or_else(|| vec![config.root().to_path_buf()]);
    let written = written_roots(values);
    let extensions = files::strings(config, SECTION, values, "extensions")?;
    Ok(Document {
        path: config.path(name),
        roots,
        written,
        extensions: if extensions.is_empty() {
            default_extensions()
        } else {
            extensions
        },
        name: name.to_string(),
    })
}

fn written_roots(values: &Values) -> Vec<String> {
    match values.get("roots").and_then(Value::as_array) {
        Some(listed) => listed
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        None => vec![".".to_string()],
    }
}

fn default_extensions() -> Vec<String> {
    DEFAULT_EXTENSIONS
        .iter()
        .map(|extension| extension.to_string())
        .collect()
}

fn identity(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
