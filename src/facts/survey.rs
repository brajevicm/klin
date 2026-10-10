//! What a repository says about itself, and nothing a check decides. The survey of one tree is
//! its source roots, the documents at its top, its test roots and its manifests. `Facts` holds
//! the derivation commit's survey beside the working tree's, read once per run. No section of
//! `klin.json` is manufactured here: each check reads these facts and resolves its own policy.
//! Spec 4.3, ADR 0038, ADR 0040.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::scope::{self, ROOT, ancestors, under_or_at};
use crate::facts::tree::Tree;
use crate::sys::cache::Cache;
use crate::{
    facts::files,
    sys::{git, state},
};

/// The key one derivation commit's survey is cached under, beside the other derivations of that
/// commit. Spec 6.6.
const KEY: &str = "survey";

/// The basenames the survey calls a manifest. A directory that directly holds one is a package
/// directory to the test roots of spec 5.4, and a check that reads one owns what it says.
const MANIFESTS: &[&str] = &["Cargo.toml", "go.mod", "package.json", "tsconfig.json"];

/// The directory segments a language's test convention uses, and the affixes that mark one file
/// as a test. Spec 8.2.
pub const TEST_DIRS: &[&str] = &["tests", "test", "spec", "__tests__"];
pub const TEST_PREFIXES: &[&str] = &["test_", "spec_"];
pub const TEST_SUFFIXES: &[&str] = &["_test", "_spec", ".test", ".spec", "Test", "Tests"];

/// What one tree says about itself. Spec 5.4.
#[derive(Default, Clone)]
pub struct Survey {
    pub roots: Vec<String>,
    pub documents: Vec<String>,
    /// The agent instruction files: `AGENTS.md` and `CLAUDE.md` at the root, and `AGENTS.md`
    /// in any directory. Spec 5.4.
    pub instructions: Vec<String>,
    pub test_roots: Vec<String>,
    pub manifests: Vec<String>,
    /// Whether the tree holds a source file its test convention marks, wherever it sits.
    pub tests: bool,
}

/// The facts of one run: the derivation commit's survey, and its union with the working tree's,
/// kept to what the working tree still holds. Spec 4.3.
pub struct Facts {
    /// The state directory the caches sit in, and `None` where klin keeps none.
    pub state: Option<PathBuf>,
    /// Whether the run writes the state directory, which a read-only command does not.
    pub keeps: bool,
    pub commit: Option<String>,
    /// The derivation commit's own survey, and `None` where there is no commit to survey.
    pub held: Option<Survey>,
    pub found: Survey,
    /// The derived roots the derivation commit's survey did not hold. A site under one and
    /// under no root the survey held matches nothing in `before`, so a directory that becomes a
    /// root brings no inherited debt with it. Empty when there is no commit to survey. Spec 7.1.
    pub unheld: Vec<String>,
}

impl Facts {
    /// Whether the derivation commit's survey held the path: under no root it did not hold, or
    /// under a root it held. A root that moves up to a crate directory keeps the debt below the
    /// root it replaced held. Spec 7.1.
    pub fn was_held(&self, file: &str) -> bool {
        let under = |roots: &[String]| roots.iter().any(|root| under_or_at(file, root));
        !under(&self.unheld) || self.held.as_ref().is_some_and(|held| under(&held.roots))
    }

    pub fn at_commit(&self) -> Option<(&Survey, &str)> {
        self.held.as_ref().zip(self.commit.as_deref())
    }

    pub fn cache(&self) -> Option<Cache<'_>> {
        self.state.as_deref().map(|at| Cache::new(at, self.keeps))
    }
}

/// The facts of a tree, read from the derivation commit's cached survey and the tree's one file
/// list. Nothing expensive is computed here. A run that does not keep state reads the state
/// directory only where it already is. Spec 4.3, 11.6.
pub fn facts(tree: &Tree, commit: Option<&str>, keeps: bool) -> Facts {
    let root = tree.root();
    let directory = match keeps {
        true => state::ready(root).ok(),
        false => state::dir(root).filter(|at| at.is_dir()),
    };
    let commit = commit.map(str::to_string);
    let cache = directory.as_deref().map(|at| Cache::new(at, keeps));
    let held = at_commit(root, cache, commit.as_deref());
    let found = union(&held.clone().unwrap_or_default(), &walked(tree), root);
    let unheld = match &held {
        Some(held) => found
            .roots
            .iter()
            .filter(|root| !held.roots.iter().any(|was| under_or_at(root, was)))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    Facts {
        state: directory,
        keeps,
        commit,
        held,
        found,
        unheld,
    }
}

/// The derivation commit's own survey, from the cache when this binary wrote it and from one
/// listing of that commit otherwise. Outside a repository there is no commit and no survey.
fn at_commit(root: &Path, cache: Option<Cache>, commit: Option<&str>) -> Option<Survey> {
    let commit = commit?;
    if let Some(held) = cache
        .and_then(|cache| cache.read(commit, KEY))
        .and_then(|held| read(&held))
    {
        return Some(held);
    }
    let found = of(&listed(root, commit)?);
    if let Some(cache) = cache {
        cache.write(commit, KEY, kept(&found));
    }
    Some(found)
}

/// Every path the commit holds, as git names them, less the ones no survey reads. `None` when
/// git could not read the commit, which is not the same as a commit that holds nothing: an
/// empty survey would discard the whole base and read every site as new. Spec 14.
pub(crate) fn listed(root: &Path, commit: &str) -> Option<Vec<String>> {
    let listed = git::Repo::at(root).ls_tree_paths(commit)?;
    Some(listed.into_iter().filter(|path| surveyed(path)).collect())
}

/// What spec 5.4 calls the tests of this tree alone, off its one file list, with the roots the
/// tree holds for the run.
pub fn tests(tree: &Tree) -> Tests {
    Tests {
        roots: tree
            .test_roots(|tree| walked(tree).test_roots.into_iter().collect())
            .clone(),
    }
}

/// Every path the working tree holds that a survey reads, off the tree's one file list.
fn walked(tree: &Tree) -> Survey {
    let files = tree.files().unwrap_or_default();
    of(&files
        .iter()
        .filter(|path| surveyed(path))
        .cloned()
        .collect::<Vec<String>>())
}

/// Whether a survey reads this path: none under the default skip set or a hidden directory.
pub fn surveyed(path: &str) -> bool {
    !path
        .split('/')
        .any(|segment| segment.starts_with('.') || files::skipped(segment))
}

fn of(paths: &[String]) -> Survey {
    let merged = merged(paths);
    let manifests = sorted(
        paths
            .iter()
            .filter(|path| MANIFESTS.contains(&basename(path)))
            .cloned(),
    );
    Survey {
        roots: outermost(&merged),
        documents: sorted(paths.iter().filter(|path| document(path)).cloned()),
        instructions: sorted(paths.iter().filter(|path| instruction(path)).cloned()),
        test_roots: test_roots(&merged, &manifests, paths),
        manifests,
        tests: paths.iter().any(|path| marked(path)),
    }
}

/// Whether the test convention of spec 8.2 marks a source file: a test directory segment above
/// it, or a test affix on its basename.
pub fn marked(path: &str) -> bool {
    source(path) && (named_for_tests(&parent(path)) || test_affix(basename(path)))
}

fn document(path: &str) -> bool {
    !path.contains('/') && path.ends_with(".md")
}

/// Whether the path is an agent instruction file. Spec 5.4.
pub fn instruction(path: &str) -> bool {
    basename(path) == "AGENTS.md" || path == "CLAUDE.md"
}

/// Spec 5.4.
fn merged(paths: &[String]) -> Vec<String> {
    let mixed = holding(paths.iter().filter(|path| !source(path)));
    let mut found = BTreeSet::new();
    for file in paths.iter().filter(|path| source(path)) {
        let mut upward = ancestors(file);
        let start = upward.next().unwrap_or(ROOT);
        let end = upward.take_while(|up| !mixed.contains(up)).last();
        found.insert(end.unwrap_or(start));
    }
    found.into_iter().map(str::to_string).collect()
}

/// Spec 5.4.
fn outermost(directories: &[String]) -> Vec<String> {
    let found: HashSet<&str> = directories.iter().map(String::as_str).collect();
    directories
        .iter()
        .filter(|at| !ancestors(at).any(|up| found.contains(up)))
        .cloned()
        .collect()
}

/// Spec 5.4.
fn holding<'a>(paths: impl Iterator<Item = &'a String>) -> HashSet<&'a str> {
    let mut found = HashSet::new();
    for path in paths {
        for up in ancestors(path) {
            if !found.insert(up) {
                break;
            }
        }
    }
    found
}

/// Whether a path is source, which is a fact of the path and no check's opinion. ADR 0038.
fn source(path: &str) -> bool {
    language_of(path).is_some()
}

/// Spec 5.4, 8.2.
fn test_roots(merged: &[String], manifests: &[String], paths: &[String]) -> Vec<String> {
    let production = holding(
        paths
            .iter()
            .filter(|path| source(path) && !test_affix(basename(path))),
    );
    let packages: HashSet<&str> = manifests
        .iter()
        .filter_map(|path| ancestors(path).next())
        .collect();
    let ends: HashSet<&str> = merged.iter().map(String::as_str).collect();
    let candidates: Vec<String> = merged
        .iter()
        .filter(|at| named_for_tests(at) || !production.contains(at.as_str()))
        .filter(|at| {
            !ancestors(at)
                .take_while(|up| !packages.contains(up))
                .any(|up| ends.contains(up))
        })
        .cloned()
        .collect();
    outermost(&candidates)
}

pub fn named_for_tests(directory: &str) -> bool {
    directory
        .split('/')
        .any(|segment| TEST_DIRS.contains(&segment))
}

/// Whether a basename carries one of the test affixes of spec 8.2.
pub fn test_affix(name: &str) -> bool {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    TEST_PREFIXES.iter().any(|prefix| stem.starts_with(prefix))
        || TEST_SUFFIXES.iter().any(|suffix| stem.ends_with(suffix))
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

pub fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ROOT.to_string(), |(at, _)| at.to_string())
}

fn sorted(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut found: Vec<String> = values.collect();
    found.sort();
    found.dedup();
    found
}

/// The two surveys as one. A path set may only grow between the trees, so this is their union,
/// less what the working tree no longer holds: klin cannot measure a root that is gone, and
/// dropping it loosens nothing, because neither tree is read there. Spec 4.3.
fn union(held: &Survey, now: &Survey, root: &Path) -> Survey {
    let both = |left: &[String], right: &[String]| sorted(left.iter().chain(right).cloned());
    let roots: Vec<String> = both(&held.roots, &now.roots)
        .into_iter()
        .filter(|at| root.join(at).is_dir())
        .collect();
    Survey {
        documents: both(&held.documents, &now.documents)
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect(),
        instructions: both(&held.instructions, &now.instructions)
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect(),
        test_roots: both(&held.test_roots, &now.test_roots)
            .into_iter()
            .filter(|at| root.join(at).is_dir())
            .collect(),
        manifests: both(&held.manifests, &now.manifests)
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect(),
        tests: held.tests || now.tests,
        roots,
    }
}

fn read(cached: &Value) -> Option<Survey> {
    Some(Survey {
        roots: names(cached, "roots")?,
        documents: names(cached, "documents")?,
        instructions: names(cached, "instructions")?,
        test_roots: names(cached, "test_roots")?,
        manifests: names(cached, "manifests")?,
        tests: cached.get("tests")?.as_bool()?,
    })
}

fn names(cached: &Value, key: &str) -> Option<Vec<String>> {
    cached
        .get(key)?
        .as_array()?
        .iter()
        .map(|name| name.as_str().map(str::to_string))
        .collect()
}

fn kept(found: &Survey) -> Value {
    let list = |values: &[String]| Value::from(values.to_vec());
    let mut fields = Map::new();
    fields.insert("roots".into(), list(&found.roots));
    fields.insert("documents".into(), list(&found.documents));
    fields.insert("instructions".into(), list(&found.instructions));
    fields.insert("test_roots".into(), list(&found.test_roots));
    fields.insert("manifests".into(), list(&found.manifests));
    fields.insert("tests".into(), found.tests.into());
    Value::Object(fields)
}

/// One language a file is classified as by its extension, which is a fact of the path and no
/// check's opinion. The escapes table names its own rows for each of these by the same name,
/// and the survey leaves out a language that table has no rows for. Spec 5.4.
pub struct Language {
    pub name: &'static str,
    pub suffixes: &'static [&'static str],
}

/// The languages the survey calls source, by extension. This is what a derived root is a
/// directory of, so it belongs below every check. Spec 5.4, ADR 0038.
pub const LANGUAGES: &[Language] = &[
    Language {
        name: "go",
        suffixes: &[".go"],
    },
    Language {
        name: "java",
        suffixes: &[".java"],
    },
    Language {
        name: "kotlin",
        suffixes: &[".kt", ".kts"],
    },
    Language {
        name: "python",
        suffixes: &[".py"],
    },
    Language {
        name: "ruby",
        suffixes: &[".rb"],
    },
    Language {
        name: "rust",
        suffixes: &[".rs"],
    },
    Language {
        name: "shell",
        suffixes: &[".sh", ".bash", ".zsh"],
    },
    Language {
        name: "swift",
        suffixes: &[".swift"],
    },
    Language {
        name: "typescript",
        suffixes: &[".ts", ".tsx", ".mts", ".cts"],
    },
    Language {
        name: "javascript",
        suffixes: &[".js", ".jsx", ".mjs", ".cjs"],
    },
];

/// The language a path is written in, by its extension, and `None` for a file no language
/// claims. This is what the survey calls source.
pub fn language_of(path: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .find(|language| {
            language
                .suffixes
                .iter()
                .any(|suffix| path.ends_with(suffix))
        })
        .map(|language| language.name)
}

/// The tests of one tree: every file under one of its test roots, and every source file a test
/// directory segment or a test affix marks, wherever it sits. Spec 5.4, 8.2.
#[derive(Clone)]
pub struct Tests {
    roots: scope::Roots,
}

impl Tests {
    /// Whether one of the tree's test roots holds this path.
    pub fn root_holds(&self, path: &str) -> bool {
        self.roots.holds(path)
    }

    /// Whether spec 5.4 calls this path a test file.
    pub fn file_holds(&self, path: &str) -> bool {
        self.root_holds(path) || marked(path)
    }
}
