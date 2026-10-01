//! TypeScript modules. Every TypeScript or TSX file the tree lists is one module. A relative
//! import or re-export resolves to the one file V1's candidate rule finds: the specifier as
//! written where it names a TypeScript extension, the TypeScript file a `.js`, `.jsx`, `.mjs` or
//! `.cjs` specifier stands for, and otherwise `.ts`, `.tsx`, `index.ts` and `index.tsx`. Two
//! candidates are a hole, never a choice. A bare, package, alias or absolute specifier is
//! counted and never resolved, and so is a relative one that names a file of another kind.

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
    let mut modules: BTreeMap<&str, usize> = BTreeMap::new();
    for file in topology.files.iter().filter(|file| source(file)) {
        let index = builder.module(file.clone(), &[file], Attachment::File);
        modules.insert(file, index);
    }
    for (file, from) in &modules {
        let Some(facts) = topology.facts(file) else {
            continue;
        };
        for import in &facts.imports {
            if let Some(specifier) = import.module.as_deref() {
                let site = Site {
                    file,
                    from: *from,
                    line: import.line,
                    text: &import.text,
                };
                resolved(builder, &modules, &site, specifier);
            }
        }
    }
}

struct Site<'a> {
    file: &'a str,
    from: usize,
    line: u64,
    text: &'a str,
}

fn resolved(builder: &mut Builder, modules: &BTreeMap<&str, usize>, site: &Site, specifier: &str) {
    let topology = builder.topology;
    let Some(base) = relative(specifier)
        .then(|| joined(directory(site.file), specifier))
        .flatten()
    else {
        builder.external += 1;
        return;
    };
    let candidates = candidates(&base);
    let held: Vec<&String> = candidates
        .iter()
        .filter(|file| modules.contains_key(file.as_str()))
        .collect();
    match held.as_slice() {
        [target] => builder.depend(site.from, modules[target.as_str()], site.file, site.line),
        [] if another_kind(topology, &base, &candidates) => builder.external += 1,
        [] => builder.hole(
            site.file,
            site.line,
            site.text,
            format!(
                "{specifier} names no TypeScript file the tree holds: {}",
                candidates.join(", ")
            ),
        ),
        _ => builder.hole(
            site.file,
            site.line,
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
