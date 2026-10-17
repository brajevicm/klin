//! TypeScript modules. Every TypeScript or TSX file the tree lists is one module. A relative
//! import or re-export resolves to the one file V1's candidate rule finds: the specifier as
//! written where it names a TypeScript extension, the TypeScript file a `.js`, `.jsx`, `.mjs` or
//! `.cjs` specifier stands for, and otherwise `.ts`, `.tsx`, `index.ts` and `index.tsx`. Two
//! candidates are a hole, never a choice. Explicit tsconfig paths aliases use that same rule.
//! Unproved local aliases are holes. A bare name that no alias places goes on to the workspace
//! packages the root lists; other packages and targets of another kind stay outside V1.

use std::collections::BTreeMap;

use super::resolver::{Attachment, Builder, Doubt, Topology, directory, joined};
use super::typescript_workspaces::Workspaces;

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
    let workspaces = Workspaces::read(topology);
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
                let graph = Graph {
                    modules: &modules,
                    workspaces: &workspaces,
                };
                resolved(builder, &graph, scope.as_ref(), &site, specifier);
            }
        }
    }
}

/// What an import resolves against: the tree's TypeScript modules and its workspace packages.
struct Graph<'a> {
    modules: &'a BTreeMap<&'a str, usize>,
    workspaces: &'a Workspaces,
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
    graph: &Graph,
    paths: Option<&super::typescript_paths::FilePaths<'_>>,
    site: &Site,
    specifier: &str,
) {
    let before = builder.holes.len();
    let modules = graph.modules;
    let reaches = |base: &str| {
        candidates(base)
            .iter()
            .any(|file| modules.contains_key(file.as_str()))
            || builder.topology.holds(&manifest(base))
    };
    match located(paths, site.file, specifier, reaches) {
        Some(Ok((base, lookup))) => {
            if !dependency(builder, modules, site, specifier, &base, lookup) {
                packaged(builder, graph, site, specifier);
            }
        }
        Some(Err(why)) => hole(builder, site, why, Doubt::Unproved),
        None => packaged(builder, graph, site, specifier),
    }
    if !relative(specifier) {
        for hole in &mut builder.holes[before..] {
            hole.local_alias = true;
        }
    }
}

/// The path a specifier names before the candidate rule, in TypeScript's order: relative, then
/// a paths rule, then a written `baseUrl`, where the first `baseUrl` that `reaches` a held file
/// wins. None is external; Err is a local name klin cannot prove.
fn located(
    paths: Option<&super::typescript_paths::FilePaths<'_>>,
    file: &str,
    specifier: &str,
    reaches: impl Fn(&str) -> bool,
) -> Option<Result<(String, Lookup), String>> {
    if relative(specifier) {
        return joined(directory(file), specifier).map(|base| Ok((base, Lookup::Named)));
    }
    let scope = paths?;
    match scope.resolve(specifier) {
        Some(found) => Some(found.map(|base| (base, Lookup::Named))),
        None => {
            let (mut bases, proved) = scope.under_base_url(specifier)?;
            let chosen = match bases.len() {
                1 => 0,
                _ => bases.iter().position(|base| reaches(base)).unwrap_or(0),
            };
            Some(Ok((bases.swap_remove(chosen), Lookup::BaseUrl { proved })))
        }
    }
}

/// How a path was named. A relative or paths name that reaches no held file is judged by its
/// form. A `baseUrl` name that reaches none goes on to packages, as TypeScript does, and one
/// that reaches a held file is a hole where klin cannot prove the config.
#[derive(Clone, Copy)]
enum Lookup {
    Named,
    BaseUrl { proved: bool },
}

impl Lookup {
    /// Why one held target is not proved: an unproved `baseUrl` config, or a directory whose
    /// `package.json` may name another entry point than its index file.
    fn doubt(
        &self,
        topology: &Topology,
        specifier: &str,
        base: &str,
        target: &str,
    ) -> Option<(String, Doubt)> {
        let Lookup::BaseUrl { proved } = self else {
            return None;
        };
        if !proved {
            return Some((
                format!(
                    "{specifier} may be a baseUrl import whose configuration klin cannot prove"
                ),
                Doubt::Unproved,
            ));
        }
        let manifest = manifest(base);
        (target.starts_with(&format!("{base}/")) && topology.holds(&manifest)).then(|| {
            (
                format!(
                    "{specifier} names a directory whose {manifest} may choose another entry point"
                ),
                Doubt::Several(vec![target.to_string()]),
            )
        })
    }

    /// Whether a name that reaches no held TypeScript file goes on to packages, as TypeScript
    /// does: a `baseUrl` miss with no `package.json` that may name an entry point, or a bare name
    /// that may be a package and names no file of another kind.
    fn tries_packages(
        &self,
        topology: &Topology,
        specifier: &str,
        base: &str,
        candidates: &[String],
    ) -> bool {
        match self {
            Lookup::BaseUrl { .. } => !topology.holds(&manifest(base)),
            Lookup::Named => {
                !relative(specifier)
                    && package_name(specifier)
                    && !another_kind(topology, base, candidates)
            }
        }
    }
}

/// The `package.json` of the directory a path names, which may name its entry point.
fn manifest(base: &str) -> String {
    format!("{base}/package.json")
}

fn hole(builder: &mut Builder, site: &Site, why: String, doubt: Doubt) {
    builder.doubted_at(
        site.file,
        (site.line, site.start_byte),
        site.text,
        why,
        doubt,
    );
}

/// Settles an import whose path klin located, and false where the name goes on to packages.
fn dependency(
    builder: &mut Builder,
    modules: &BTreeMap<&str, usize>,
    site: &Site,
    specifier: &str,
    base: &str,
    lookup: Lookup,
) -> bool {
    let topology = builder.topology;
    let candidates = candidates(base);
    let held: Vec<&String> = candidates
        .iter()
        .filter(|file| modules.contains_key(file.as_str()))
        .collect();
    match held.as_slice() {
        [target] => match lookup.doubt(topology, specifier, base, target) {
            Some((why, doubt)) => hole(builder, site, why, doubt),
            None => depend(builder, modules, site, target),
        },
        [] if lookup.tries_packages(topology, specifier, base, &candidates) => return false,
        [] if matches!(lookup, Lookup::Named) && another_kind(topology, base, &candidates) => {
            builder.external += 1
        }
        [] => hole(
            builder,
            site,
            format!(
                "{specifier} names no TypeScript file the tree holds: {}",
                candidates.join(", ")
            ),
            Doubt::Missing,
        ),
        _ => several(builder, site, specifier, &held),
    }
    true
}

/// The workspace step: a name of one workspace package reaches the first of its paths that
/// names a held file, and anything klin cannot place stays external and makes no hole.
fn packaged(builder: &mut Builder, graph: &Graph, site: &Site, specifier: &str) {
    let held = graph
        .workspaces
        .bases(specifier)
        .iter()
        .map(|base| candidates(base))
        .map(|candidates| {
            candidates
                .into_iter()
                .filter(|file| graph.modules.contains_key(file.as_str()))
                .collect::<Vec<_>>()
        })
        .find(|held| !held.is_empty())
        .unwrap_or_default();
    match held.iter().collect::<Vec<_>>().as_slice() {
        [] => builder.external += 1,
        [target] => depend(builder, graph.modules, site, target),
        several_held => several(builder, site, specifier, several_held),
    }
}

fn depend(builder: &mut Builder, modules: &BTreeMap<&str, usize>, site: &Site, target: &str) {
    builder.depend_at(
        site.from,
        modules[target],
        site.file,
        site.line,
        site.start_byte,
    );
}

fn several(builder: &mut Builder, site: &Site, specifier: &str, held: &[&String]) {
    hole(
        builder,
        site,
        format!(
            "{specifier} names more than one TypeScript file: {}",
            held.iter()
                .map(|file| file.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Doubt::Several(held.iter().map(|file| file.to_string()).collect()),
    );
}

fn source(file: &str) -> bool {
    crate::syntax::structural::language_of(file) == Some(crate::syntax::LanguageId::TypeScript)
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
