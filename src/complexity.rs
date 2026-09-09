use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tree_sitter::{Node, Parser};

use crate::base;
use crate::config::{Config, Error, Flags};
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};

const SECTION: &str = "complexity";

struct Language {
    name: &'static str,
    names: &'static [&'static str],
    extensions: &'static [&'static str],
    grammar: fn() -> tree_sitter::Language,
    functions: &'static [&'static str],
    decisions: &'static [&'static str],
    operators: &'static [&'static str],
}

fn rust() -> tree_sitter::Language {
    tree_sitter_rust::LANGUAGE.into()
}

fn python() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

fn typescript() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

fn tsx() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TSX.into()
}

fn javascript() -> tree_sitter::Language {
    tree_sitter_javascript::LANGUAGE.into()
}

fn go() -> tree_sitter::Language {
    tree_sitter_go::LANGUAGE.into()
}

fn java() -> tree_sitter::Language {
    tree_sitter_java::LANGUAGE.into()
}

fn ruby() -> tree_sitter::Language {
    tree_sitter_ruby::LANGUAGE.into()
}

fn swift() -> tree_sitter::Language {
    tree_sitter_swift::LANGUAGE.into()
}

fn kotlin() -> tree_sitter::Language {
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

const ECMASCRIPT_FUNCTIONS: &[&str] = &[
    "function_declaration",
    "function_expression",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
];

const ECMASCRIPT_DECISIONS: &[&str] = &[
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "for_in_statement",
    "switch_case",
    "catch_clause",
    "ternary_expression",
];

const ECMASCRIPT_OPERATORS: &[&str] = &["&&", "||", "??"];

const ACCESSOR_HOLDERS: &[&str] = &[
    "computed_property",
    "subscript_declaration",
    "property_declaration",
    "willset_didset_block",
];

const FALL_THROUGH_ARMS: &[(&str, &str)] = &[
    ("switch_label", "default"),
    ("switch_entry", "default_keyword"),
    ("when_entry", "else"),
];

const CATCH_ALL_PATTERNS: &[&str] = &["match_pattern", "case_pattern"];

const LANGUAGES: &[Language] = &[
    Language {
        name: "Rust",
        names: &["rust"],
        extensions: &[".rs"],
        grammar: rust,
        functions: &["function_item"],
        decisions: &[
            "if_expression",
            "while_expression",
            "loop_expression",
            "for_expression",
            "match_arm",
            "try_expression",
        ],
        operators: &["&&", "||"],
    },
    Language {
        name: "Python",
        names: &["python"],
        extensions: &[".py"],
        grammar: python,
        functions: &["function_definition"],
        decisions: &[
            "if_statement",
            "elif_clause",
            "while_statement",
            "for_statement",
            "for_in_clause",
            "if_clause",
            "except_clause",
            "case_clause",
            "conditional_expression",
            "boolean_operator",
            "assert_statement",
        ],
        operators: &[],
    },
    Language {
        name: "TypeScript",
        names: &["typescript"],
        extensions: &[".ts", ".mts", ".cts"],
        grammar: typescript,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "TSX",
        names: &["typescript", "tsx"],
        extensions: &[".tsx"],
        grammar: tsx,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "JavaScript",
        names: &["javascript"],
        extensions: &[".js", ".jsx", ".mjs", ".cjs"],
        grammar: javascript,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "Go",
        names: &["go"],
        extensions: &[".go"],
        grammar: go,
        functions: &["function_declaration", "method_declaration", "func_literal"],
        decisions: &[
            "if_statement",
            "for_statement",
            "expression_case",
            "type_case",
            "communication_case",
        ],
        operators: &["&&", "||"],
    },
    Language {
        name: "Java",
        names: &["java"],
        extensions: &[".java"],
        grammar: java,
        functions: &[
            "method_declaration",
            "constructor_declaration",
            "compact_constructor_declaration",
            "static_initializer",
            "lambda_expression",
        ],
        decisions: &[
            "if_statement",
            "while_statement",
            "do_statement",
            "for_statement",
            "enhanced_for_statement",
            "switch_label",
            "catch_clause",
            "ternary_expression",
        ],
        operators: &["&&", "||"],
    },
    Language {
        name: "Ruby",
        names: &["ruby"],
        extensions: &[".rb"],
        grammar: ruby,
        functions: &["method", "singleton_method"],
        decisions: &[
            "if",
            "elsif",
            "unless",
            "while",
            "until",
            "for",
            "when",
            "in_clause",
            "rescue",
            "conditional",
            "if_modifier",
            "unless_modifier",
            "while_modifier",
            "until_modifier",
            "rescue_modifier",
        ],
        operators: &["&&", "||", "and", "or"],
    },
    Language {
        name: "Swift",
        names: &["swift"],
        extensions: &[".swift"],
        grammar: swift,
        functions: &[
            "function_declaration",
            "init_declaration",
            "deinit_declaration",
            "subscript_declaration",
            "computed_property",
            "computed_getter",
            "computed_setter",
            "willset_clause",
            "didset_clause",
        ],
        decisions: &[
            "if_statement",
            "guard_statement",
            "while_statement",
            "repeat_while_statement",
            "for_statement",
            "switch_entry",
            "catch_block",
            "ternary_expression",
            "conjunction_expression",
            "disjunction_expression",
            "nil_coalescing_expression",
        ],
        operators: &[],
    },
    Language {
        name: "Kotlin",
        names: &["kotlin"],
        extensions: &[".kt", ".kts"],
        grammar: kotlin,
        functions: &[
            "function_declaration",
            "anonymous_function",
            "secondary_constructor",
            "anonymous_initializer",
            "getter",
            "setter",
        ],
        decisions: &[
            "if_expression",
            "when_entry",
            "while_statement",
            "do_while_statement",
            "for_statement",
            "catch_block",
        ],
        operators: &["&&", "||", "?:"],
    },
];

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
    /// Judge only these repo-relative files, against only their functions at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

struct Function {
    file: String,
    line: u64,
    end: u64,
    cc: u64,
    text: String,
}

impl Function {
    fn length(&self) -> u64 {
        self.end - self.line + 1
    }

    fn over(&self, ceilings: &Ceilings) -> bool {
        self.cc > ceilings.cc || self.length() > ceilings.lines
    }

    fn finding(&self) -> Finding {
        let mut values = Values::new();
        values.insert("cc".into(), self.cc.into());
        values.insert("lines".into(), self.length().into());
        Finding {
            file: self.file.clone(),
            line: self.line,
            text: self.text.clone(),
            values,
        }
    }
}

struct Ceilings {
    cc: u64,
    lines: u64,
}

struct Unparsed {
    file: String,
    language: &'static str,
}

struct Selection {
    languages: Vec<&'static Language>,
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
    exclude_except: Vec<PathBuf>,
}

struct Spec {
    sources: Vec<PathBuf>,
    selection: Selection,
    ceilings: Ceilings,
    gate_text: String,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(&flags(args), start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, start, out)
}

fn evaluate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    let spec = spec(&config)?;
    let (functions, unparsed) = measure(&spec.sources, &spec.selection, config.root())?;
    let now = over(&functions, &spec);
    let judged = scoped(functions.iter().map(|function| &function.file), flags);
    let count = scoped(now.iter().map(|finding| &finding.file), flags);
    let code = evaluator(&spec).evaluate(
        now,
        at_the_base(&config, &spec, flags, out)?,
        ratchet::accepted(&config, &flags.gate, evaluator(&spec).metrics)?,
        flags,
        &format!("OK: {judged} function(s) judged, {count} over the gate, all held at the base"),
        out,
    );
    Ok(unread(&unparsed, flags, code, out))
}

fn at_the_base(
    config: &Config,
    spec: &Spec,
    flags: &Flags,
    out: &mut String,
) -> Result<Vec<Finding>, Error> {
    let owned;
    let prior = match flags.prior.as_deref() {
        Some(dir) => dir,
        None => {
            owned = base::own(config, flags, out)?;
            owned.root()
        }
    };
    let (before, _) = measure(
        &base::roots(&spec.sources, config, prior)?,
        &spec.selection,
        prior,
    )?;
    Ok(over(&before, spec))
}

fn over(functions: &[Function], spec: &Spec) -> Vec<Finding> {
    functions
        .iter()
        .filter(|function| function.over(&spec.ceilings))
        .map(Function::finding)
        .collect()
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: SECTION.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        strict: args.strict,
        only: args.only.clone(),
        records: None,
        with: None,
    }
}

fn scoped<'a>(files: impl Iterator<Item = &'a String>, flags: &Flags) -> usize {
    match flags.only.as_deref() {
        Some(only) => files.filter(|file| only.contains(file)).count(),
        None => files.count(),
    }
}

fn unread(unparsed: &[Unparsed], flags: &Flags, code: u8, out: &mut String) -> u8 {
    let only = flags.only.as_deref();
    let named: Vec<&Unparsed> = unparsed
        .iter()
        .filter(|file| only.is_none_or(|only| only.contains(&file.file)))
        .collect();
    if named.is_empty() {
        return code;
    }
    let _ = writeln!(
        out,
        "FAIL: {} file(s) the grammar could not parse, so nothing in them was measured:",
        named.len()
    );
    for file in named {
        let rejected = format!("the {} grammar rejected it", file.language);
        let _ = writeln!(out, "  {}  {rejected}", file.file);
        flags.record(|records| {
            let mut out = Values::new();
            out.insert("outcome".into(), "unparsed".into());
            out.insert("file".into(), file.file.clone().into());
            out.insert("text".into(), rejected.clone().into());
            records.findings.push(Value::Object(out));
        });
    }
    let _ = writeln!(
        out,
        "A file klin cannot read is a hole in the ratchet. Update the grammar, or exclude \
         the file and accept that nothing measures it."
    );
    2
}

fn evaluator(spec: &Spec) -> Evaluator<'_> {
    Evaluator {
        metrics: &["cc", "lines"],
        unit: "function(s)",
        condition: &spec.gate_text,
        fix_advice: "Split the function so each piece is under the gate. Accepting new debt is a \
                     policy decision for a person, in the config, in a reviewed commit.",
        format_metrics: show,
    }
}

fn spec(config: &Config) -> Result<Spec, Error> {
    let section = ratchet::section(config, SECTION)?;
    let values = &section.values;
    let ceilings = ceilings(section.config, values)?;
    Ok(Spec {
        sources: files::roots(section.config, section.name, values, "sources")?
            .ok_or_else(|| section.config.missing(section.name, "sources"))?,
        selection: selection(section.config, values)?,
        gate_text: format!(
            "over the complexity gate (cyclomatic > {} or body > {} lines)",
            ceilings.cc, ceilings.lines
        ),
        ceilings,
    })
}

fn selection(config: &Config, section: &Values) -> Result<Selection, Error> {
    let named = files::strings(config, SECTION, section, "languages")?;
    Ok(Selection {
        languages: languages(config, &named)?,
        skip_dirs: files::skip_dirs(config, SECTION, section)?,
        exclude: files::strings(config, SECTION, section, "exclude")?,
        exclude_except: files::roots(config, SECTION, section, "exclude_except")?
            .unwrap_or_default(),
    })
}

fn languages(config: &Config, named: &[String]) -> Result<Vec<&'static Language>, Error> {
    if named.is_empty() {
        return Ok(LANGUAGES.iter().collect());
    }
    let mut out: Vec<&'static Language> = Vec::new();
    for name in named {
        let matching = LANGUAGES
            .iter()
            .filter(|language| language.names.contains(&name.as_str()));
        let mut found = false;
        for language in matching {
            found = true;
            if !out.iter().any(|held| std::ptr::eq(*held, language)) {
                out.push(language);
            }
        }
        if !found {
            return Err(unknown_language(config, name));
        }
    }
    Ok(out)
}

fn unknown_language(config: &Config, name: &str) -> Error {
    let mut known: Vec<&str> = LANGUAGES
        .iter()
        .flat_map(|language| language.names.iter().copied())
        .collect();
    known.sort_unstable();
    known.dedup();
    Error(format!(
        "{}: \"{SECTION}\" measures no language called \"{name}\" — one of: {}",
        config.file.display(),
        known.join(", ")
    ))
}

fn ceilings(config: &Config, section: &Values) -> Result<Ceilings, Error> {
    let listed = section
        .get("ceilings")
        .ok_or_else(|| config.missing(SECTION, "ceilings"))?
        .as_object()
        .ok_or_else(|| {
            config.malformed(SECTION, "ceilings", "an object of \"cc\" and \"lines\"")
        })?;
    let ceiling = |key: &str| {
        let named = format!("ceilings.{key}");
        listed
            .get(key)
            .ok_or_else(|| config.missing(SECTION, &named))?
            .as_u64()
            .ok_or_else(|| config.malformed(SECTION, &named, "a whole number"))
    };
    Ok(Ceilings {
        cc: ceiling("cc")?,
        lines: ceiling("lines")?,
    })
}

fn measure(
    sources: &[PathBuf],
    selection: &Selection,
    repo_root: &Path,
) -> Result<(Vec<Function>, Vec<Unparsed>), Error> {
    let extensions: Vec<&str> = selection
        .languages
        .iter()
        .flat_map(|language| language.extensions)
        .copied()
        .collect();
    let mut out = Vec::new();
    let mut unparsed = Vec::new();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &selection.skip_dirs,
        exclude: &selection.exclude,
        exclude_except: &selection.exclude_except,
        skip_hidden: true,
    };
    for file in files::under(sources, &wanted)? {
        let name = file.to_string_lossy().to_string();
        let Some(language) = selection.languages.iter().find(|language| {
            language
                .extensions
                .iter()
                .any(|extension| name.ends_with(extension))
        }) else {
            continue;
        };
        out.extend(functions(&file, repo_root, language, &mut unparsed)?);
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok((out, unparsed))
}

fn functions(
    path: &Path,
    repo_root: &Path,
    language: &Language,
    unparsed: &mut Vec<Unparsed>,
) -> Result<Vec<Function>, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    let source = String::from_utf8_lossy(&bytes).to_string();
    let mut parser = Parser::new();
    parser.set_language(&(language.grammar)()).map_err(|why| {
        Error(format!(
            "the {} grammar could not be loaded: {why}",
            language.name
        ))
    })?;
    let file = files::relative(path, repo_root);
    let tree = parser.parse(&source, None);
    let Some(tree) = tree.filter(|tree| !tree.root_node().has_error()) else {
        unparsed.push(Unparsed {
            file,
            language: language.name,
        });
        return Ok(Vec::new());
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    collect(tree.root_node(), language, &file, &lines, &mut out);
    Ok(out)
}

fn collect(node: Node, language: &Language, file: &str, lines: &[&str], out: &mut Vec<Function>) {
    if language.functions.contains(&node.kind()) && !holds_a_body(node, language) {
        out.push(Function {
            file: file.to_string(),
            line: node.start_position().row as u64 + 1,
            end: node.end_position().row as u64 + 1,
            cc: 1 + decisions(node, language),
            text: site(node, lines),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, language, file, lines, out);
    }
}

fn decisions(node: Node, language: &Language) -> u64 {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if language.functions.contains(&child.kind()) {
            continue;
        }
        let table = if child.is_named() {
            language.decisions
        } else {
            language.operators
        };
        count += u64::from(table.contains(&child.kind()) && !falls_through(child))
            + decisions(child, language);
    }
    count
}

fn holds_a_body(node: Node, language: &Language) -> bool {
    ACCESSOR_HOLDERS.contains(&node.kind())
        && has_child(node, |kind| language.functions.contains(&kind))
}

fn falls_through(node: Node) -> bool {
    FALL_THROUGH_ARMS
        .iter()
        .any(|(arm, marker)| node.kind() == *arm && has_child(node, |kind| kind == *marker))
        || catches_all(node)
}

fn catches_all(node: Node) -> bool {
    let mut cursor = node.walk();
    let mut patterns = node
        .children(&mut cursor)
        .filter(|child| CATCH_ALL_PATTERNS.contains(&child.kind()));
    let Some(pattern) = patterns.next() else {
        return false;
    };
    patterns.next().is_none()
        && pattern.child_count() == 1
        && pattern.child(0).is_some_and(|only| only.kind() == "_")
}

fn site(node: Node, lines: &[&str]) -> String {
    let row = node.start_position().row;
    match holder_row(node) {
        Some(holder) if holder < row => {
            format!("{} {}", line_at(lines, holder), line_at(lines, row))
        }
        _ => line_at(lines, row),
    }
}

fn holder_row(node: Node) -> Option<usize> {
    let mut row = None;
    let mut above = node.parent();
    while let Some(holder) = above.filter(|above| ACCESSOR_HOLDERS.contains(&above.kind())) {
        row = Some(holder.start_position().row);
        above = holder.parent();
    }
    row
}

fn line_at(lines: &[&str], row: usize) -> String {
    lines.get(row).unwrap_or(&"").trim().to_string()
}

fn has_child(node: Node, wanted: impl Fn(&str) -> bool) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| wanted(child.kind()))
}

fn show(values: &Values) -> String {
    let number = |key: &str| {
        values
            .get(key)
            .and_then(Value::as_u64)
            .map_or("?".to_string(), |value| value.to_string())
    };
    format!("cc {}, {} lines", number("cc"), number("lines"))
}
