//! TypeScript modules. Every TypeScript or TSX file the tree lists is one module. A relative
//! import or re-export resolves to the one file V1's candidate rule finds: the specifier as
//! written where it names a TypeScript extension, the TypeScript file a `.js`, `.jsx`, `.mjs` or
//! `.cjs` specifier stands for, and otherwise `.ts`, `.tsx`, `index.ts` and `index.tsx`. Two
//! candidates are a hole, never a choice. Explicit tsconfig paths aliases use that same rule.
//! Unproved local aliases are holes; packages and targets of another kind stay outside V1.

use std::collections::BTreeMap;

use super::resolver::{Attachment, Builder, Topology, directory, joined};

const SOURCE: &[&str] = &[".ts", ".tsx", ".mts", ".cts"];

/// Each script extension with the TypeScript extensions a specifier written with it stands for.
const STANDS_FOR: &[(&str, &[&str])] = &[
    (".js", &[".ts", ".tsx"]),
    (".jsx", &[".tsx"]),
    (".mjs", &[".mts"]),
    (".cjs", &[".cts"]),
];

const WITHOUT: &[&str] = &[".ts", ".tsx", "/index.ts", "/index.tsx"];

pub(super) fn resolve(builder: &mut Builder) {
    let topology = builder.topology;
    let paths = super::typescript_paths::Paths::read(topology);
    let mut modules: BTreeMap<&str, usize> = BTreeMap::new();
    for file in topology.files.iter().filter(|file| source(file)) {
        let index = builder.module(file.clone(), &[file], Attachment::File);
        modules.insert(file, index);
    }
    for (file, from) in &modules {
        let Some(facts) = topology.facts(file) else {
            continue;
        };
        let scope = paths.for_file(file);
        for import in &facts.imports {
            if let Some(specifier) = import.module.as_deref() {
                let site = Site {
                    file,
                    from: *from,
                    line: import.line,
                    start_byte: import.start_byte,
                    text: &import.text,
                };
                resolved(builder, &modules, scope, &site, specifier);
            }
        }
    }
}

struct Site<'a> {
    file: &'a str,
    from: usize,
    line: u64,
    start_byte: u64,
    text: &'a str,
}

fn resolved(
    builder: &mut Builder,
    modules: &BTreeMap<&str, usize>,
    paths: Option<super::typescript_paths::FilePaths<'_>>,
    site: &Site,
    specifier: &str,
) {
    let before = builder.holes.len();
    let base = if relative(specifier) {
        joined(directory(site.file), specifier).map(Ok)
    } else {
        paths.and_then(|scope| scope.resolve(specifier))
    };
    match base {
        Some(Ok(base)) => dependency(builder, modules, site, specifier, &base),
        Some(Err(why)) => builder.hole_at(site.file, site.line, site.start_byte, site.text, why),
        None => builder.external += 1,
    }
    if !relative(specifier) {
        for hole in &mut builder.holes[before..] {
            hole.local_alias = true;
        }
    }
}

fn dependency(
    builder: &mut Builder,
    modules: &BTreeMap<&str, usize>,
    site: &Site,
    specifier: &str,
    base: &str,
) {
    let topology = builder.topology;
    let candidates = candidates(base);
    let held: Vec<&String> = candidates
        .iter()
        .filter(|file| modules.contains_key(file.as_str()))
        .collect();
    match held.as_slice() {
        [target] => builder.depend_at(
            site.from,
            modules[target.as_str()],
            site.file,
            site.line,
            site.start_byte,
        ),
        [] if another_kind(topology, base, &candidates) => builder.external += 1,
        [] if !relative(specifier) && package_name(specifier) => builder.external += 1,
        [] => builder.hole_at(
            site.file,
            site.line,
            site.start_byte,
            site.text,
            format!(
                "{specifier} names no TypeScript file the tree holds: {}",
                candidates.join(", ")
            ),
        ),
        _ => builder.hole_at(
            site.file,
            site.line,
            site.start_byte,
            site.text,
            format!(
                "{specifier} names more than one TypeScript file: {}",
                held.iter()
                    .map(|file| file.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
    }
}

fn source(file: &str) -> bool {
    SOURCE.iter().any(|end| file.ends_with(end))
}

fn package_name(specifier: &str) -> bool {
    let named = |part: &str| {
        part.starts_with(|c: char| c.is_ascii_alphanumeric())
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
    };
    let mut parts = specifier.split('/');
    match parts.next() {
        Some(scope) if scope.starts_with('@') => {
            named(&scope[1..]) && parts.next().is_some_and(named)
        }
        Some(name) => named(name),
        None => false,
    }
}

fn relative(specifier: &str) -> bool {
    matches!(specifier, "." | "..") || specifier.starts_with("./") || specifier.starts_with("../")
}

/// The TypeScript files a relative specifier may name, joined to the importing file's directory.
fn candidates(base: &str) -> Vec<String> {
    if source(base) {
        return vec![base.to_string()];
    }
    if let Some((stem, ends)) = STANDS_FOR
        .iter()
        .find_map(|(written, ends)| Some((base.strip_suffix(written)?, *ends)))
    {
        return ends.iter().map(|end| format!("{stem}{end}")).collect();
    }
    WITHOUT.iter().map(|end| format!("{base}{end}")).collect()
}

/// Whether a specifier that names no TypeScript file names something V1 does not resolve: a
/// file of another kind, a script file, or a TypeScript file the file list leaves out.
fn another_kind(topology: &Topology, base: &str, candidates: &[String]) -> bool {
    let scripts = ["", ".js", "/index.js"].map(|end| format!("{base}{end}"));
    scripts
        .iter()
        .any(|file| topology.holds(file) || topology.ignored(file))
        || candidates.iter().any(|file| topology.ignored(file))
}
