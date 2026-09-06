use std::path::{Path, PathBuf};

use serde_json::Value;
use tree_sitter::{Node, Parser};

use crate::config::{Config, Error};
use crate::files;
use crate::ratchet::{self, Finding, Gate, Values};

const SECTION: &str = "complexity";
const VERSION: &str = "1";

struct Language {
    name: &'static str,
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

const LANGUAGES: &[Language] = &[
    Language {
        name: "Rust",
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
        extensions: &[".ts", ".mts", ".cts"],
        grammar: typescript,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "TSX",
        extensions: &[".tsx"],
        grammar: tsx,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "JavaScript",
        extensions: &[".js", ".jsx", ".mjs", ".cjs"],
        grammar: javascript,
        functions: ECMASCRIPT_FUNCTIONS,
        decisions: ECMASCRIPT_DECISIONS,
        operators: ECMASCRIPT_OPERATORS,
    },
    Language {
        name: "Go",
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
    /// The quality.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when the baseline is looser than the code — what CI runs
    #[arg(long)]
    strict: bool,
    /// Accept every function that is over the gate today
    #[arg(long)]
    write_baseline: bool,
    /// Judge only these repo-relative files, against only their baseline entries
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

struct Spec {
    sources: Vec<PathBuf>,
    ceilings: Ceilings,
    baseline: PathBuf,
    measured: Values,
    gate_text: String,
}

pub fn run(args: &Args, start: &Path) -> Result<u8, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let spec = spec(&config)?;
    let functions = measure(&spec.sources, config.root())?;
    let over: Vec<Finding> = functions
        .iter()
        .filter(|function| function.over(&spec.ceilings))
        .map(Function::finding)
        .collect();

    if args.write_baseline {
        ratchet::write(&spec.baseline, &over, &spec.measured)?;
        println!(
            "baseline written: {} function(s) {}",
            over.len(),
            spec.gate_text
        );
        return Ok(0);
    }
    assess(args, &spec, over, &functions)
}

fn assess(
    args: &Args,
    spec: &Spec,
    over: Vec<Finding>,
    measured: &[Function],
) -> Result<u8, Error> {
    let judged = match args.only.as_deref() {
        Some(only) => measured
            .iter()
            .filter(|function| only.contains(&function.file))
            .count(),
        None => measured.len(),
    };
    let (entries, stored) = ratchet::read(&spec.baseline)?;
    let (over, entries) = ratchet::restrict(over, entries, args.only.as_deref());
    let count = over.len();
    let baseline_size = entries.len();
    let verdict = ratchet::judge(
        over,
        entries,
        &["cc", "lines"],
        stored.as_ref(),
        Some(&spec.measured),
    );
    let gate = Gate {
        noun: "function(s)",
        over: &spec.gate_text,
        fix: "Split the function so each piece is under the gate. Accepting new debt into the \
              baseline is a policy decision for a person, not a fix.",
        remedy: "detent complexity --write-baseline",
        show,
    };
    let ok_line =
        format!("OK: {judged} function(s) judged, {count} over the gate, all in the baseline");
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

fn spec(config: &Config) -> Result<Spec, Error> {
    let Some(section) = config.section(SECTION)?.as_object() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be an object",
            config.file.display()
        )));
    };
    let ceilings = ceilings(config, section)?;
    let mut gate_config = section.clone();
    gate_config.remove("baseline");
    Ok(Spec {
        baseline: ratchet::baseline_path(config, SECTION, section)?,
        sources: files::roots(config, SECTION, section, "sources")?
            .ok_or_else(|| config.missing(SECTION, "sources"))?,
        gate_text: format!(
            "over the complexity gate (cyclomatic > {} or body > {} lines)",
            ceilings.cc, ceilings.lines
        ),
        ceilings,
        measured: ratchet::provenance(SECTION, VERSION, &Value::Object(gate_config)),
    })
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

fn measure(sources: &[PathBuf], repo_root: &Path) -> Result<Vec<Function>, Error> {
    let extensions: Vec<&str> = LANGUAGES
        .iter()
        .flat_map(|language| language.extensions)
        .copied()
        .collect();
    let mut out = Vec::new();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &[],
        exclude: &[],
        skip_hidden: true,
    };
    for file in files::under(sources, &wanted)? {
        let name = file.to_string_lossy().to_string();
        let Some(language) = LANGUAGES.iter().find(|language| {
            language
                .extensions
                .iter()
                .any(|extension| name.ends_with(extension))
        }) else {
            continue;
        };
        out.extend(functions(&file, repo_root, language)?);
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(out)
}

fn functions(path: &Path, repo_root: &Path, language: &Language) -> Result<Vec<Function>, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    let source = String::from_utf8_lossy(&bytes).to_string();
    let mut parser = Parser::new();
    parser.set_language(&(language.grammar)()).map_err(|why| {
        Error(format!(
            "the {} grammar could not be loaded: {why}",
            language.name
        ))
    })?;
    let tree = parser.parse(&source, None).ok_or_else(|| {
        Error(format!(
            "{}: the {} parser produced no tree",
            path.display(),
            language.name
        ))
    })?;
    if tree.root_node().has_error() {
        return Err(Error(format!(
            "{}: the {} grammar could not parse this file, so its functions cannot be measured",
            path.display(),
            language.name
        )));
    }
    let lines: Vec<&str> = source.lines().collect();
    let file = files::relative(path, repo_root);
    let mut out = Vec::new();
    collect(tree.root_node(), language, &file, &lines, &mut out);
    Ok(out)
}

fn collect(node: Node, language: &Language, file: &str, lines: &[&str], out: &mut Vec<Function>) {
    if language.functions.contains(&node.kind()) {
        let line = node.start_position().row as u64 + 1;
        out.push(Function {
            file: file.to_string(),
            line,
            end: node.end_position().row as u64 + 1,
            cc: 1 + decisions(node, language),
            text: lines
                .get(line as usize - 1)
                .unwrap_or(&"")
                .trim()
                .to_string(),
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
        count += u64::from(table.contains(&child.kind())) + decisions(child, language);
    }
    count
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
