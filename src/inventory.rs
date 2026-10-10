//! `inventory` ratchets the existence of tests. A test file is a file the base holds that the
//! test convention of spec 8.2 marks, keyed by its path; a test function is a site of ADR 0008
//! inside one, keyed by file and declaration line. Its one value, `missing`, rises from 0 to 1
//! when the working tree no longer holds the site. The tests are derived: every file under a
//! test root the survey finds, and every source file a test directory segment or a test affix
//! marks, narrowed only by `in` and `except` under the scope the base records. Spec 5.4, 8.2,
//! 8.2.1, ADR 0040.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::base::Prior;
use crate::check::contract::{self, Context, Counted, Line, Listed, Sink};
use crate::check::holes;
use crate::coverage::{self, Coverage};
use crate::error::Error;
use crate::git::Repo;
use crate::key::Key;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::Values;
use crate::scope::{self, Roots, Scope};
use crate::survey::{self, TEST_DIRS, TEST_PREFIXES, TEST_SUFFIXES};
use crate::syntax::convention::{self, Test};
use crate::syntax::{self, Unparsed};
use crate::tree::Tree;
use crate::turn;

pub const SECTION: &str = "inventory";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

const TEST_ROOTS: &str = "test roots";
const ROOTS_RULE: &str = "the roots that match a language's test convention";
const LABEL: &str = "test file";
const MISSING: &str = "missing";
/// The question the hook's block puts to the agent. Removing a test is ordinary work, and
/// deleting a failing one is the cheapest route to green, so klin asks once and does not judge
/// the answer. Spec 8.2.
const REMEDY: &str = "klin asks once about each test that went: say why in your reply and stop \
    again.";

/// The one affix table of spec 8.2, which the survey reads to find a test root and this gate
/// reads to name a test file's subject. Printed with the NOTE.
const RULE: &str = "the affix table: a test_ or spec_ prefix, a _test, _spec, .test or .spec \
    suffix, a Test or Tests suffix on the basename, and a tests/, test/, spec/ or __tests__/ \
    directory segment";

/// What this gate calls a test: every file under a test root the survey found, and every
/// source file a test directory segment or a test affix marks, within the section's scope.
/// Spec 5.4, 8.2.
struct Tests {
    roots: Roots,
    scope: Scope,
}

impl Tests {
    fn holds(&self, path: &str) -> bool {
        self.scope.selects(path)
            && survey::surveyed(path)
            && (self.roots.holds(path) || survey::marked(path))
    }
}

/// Whether the derivation commit or the working tree holds a test this gate judges, which is
/// when it runs with no section. A test only the commit holds is one a window may have deleted.
pub fn applies(project: &Project) -> bool {
    let found = &project.facts().found;
    !found.test_roots.is_empty() || found.tests
}

/// One test file the base holds: whether the working tree still has it, and the subject that
/// went with it, if the same window deleted one.
struct Site {
    path: String,
    gone: bool,
    subject: Option<String>,
}

/// One test function the base holds: where it was declared, whether the working tree still
/// holds it by site or by body, and whether the file that held it went in the same window.
/// The second identity of spec 8.2.
struct Function {
    site: Test,
    gone: bool,
    file_went: bool,
}

impl Function {
    /// Whether this is the NOTE of spec 8.2: the function went and so did its one subject, the
    /// file that held it, which #45 judges instead.
    fn orphaned(&self) -> bool {
        self.gone && self.file_went
    }
}

/// What the function level of this gate measured: every test function the base holds, and the
/// files in the working tree no grammar read. ADR 0003, spec 7.2.
struct Measured {
    functions: Vec<Function>,
    unparsed: Vec<Unparsed>,
}

/// One tree walked for test functions: the sites it holds, and the files no grammar read.
struct Walk {
    tests: Vec<Test>,
    unparsed: Vec<Unparsed>,
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let config = &project.config;
    let today = today(project)?;
    out.tell_each(derive(project)?);
    let commit = contract::base_commit(config.root(), at)?;
    let Found {
        judged,
        mut paired,
        measured,
    } = found(at, &commit, &today)?;
    let (mut orphans, functions): (Vec<Function>, Vec<Function>) =
        measured.functions.into_iter().partition(Function::orphaned);
    if let Some(only) = at.only {
        paired.retain(|site| only.contains(&site.path));
        orphans.retain(|function| only.contains(&function.site.file));
    }
    let now = findings(&judged, &functions);
    let (before, went) = let_through(&now, at, config.root());
    let held = ratchet::scoped(&now, at.only);
    let accepted = ratchet::accepted(config, at.gate, evaluator().metrics)?;
    let said = out.covered(&covered(&judged, &paired, &measured.unparsed, at));
    let state = Counted::Tests {
        held,
        gone: went.len(),
    };
    let code = evaluator().evaluate(now, before, accepted, at, Line::new(state, said), out);
    deleted(&went, at, out);
    noted(&paired, out);
    orphaned(&orphans, out);
    holes::unread_said(&measured.unparsed, at, out);
    Ok(code)
}

/// What the base holds of tests: the test files it judges, the ones whose subject went too, and
/// every test function with what the working tree says about it.
struct Found {
    judged: Vec<Site>,
    paired: Vec<Site>,
    measured: Measured,
}

/// The base laid out, when the runner did not lay it out already, and read under the scope it
/// records, which is also the scope the working tree is read under.
fn found(at: &Context, commit: &str, today: &Scope) -> Result<Found, Error> {
    let project = at.project;
    let root = project.config.root();
    let lay = contract::Lay::At(commit);
    contract::at_base(at, lay, (SECTION, today), |prior, scope| {
        let tests = Tests {
            roots: Roots::new(&project.facts().found.test_roots),
            scope,
        };
        let listed = at_the_base(root, commit)?;
        let (judged, paired) = sites(&tests, &listed, root)
            .into_iter()
            .partition(|site| site.subject.is_none());
        Ok(Found {
            judged,
            paired,
            measured: tests_of(&tests, at, prior)?,
        })
    })
}

/// One finding per test file and per test function the base holds.
fn findings(judged: &[Site], functions: &[Function]) -> Vec<Finding> {
    judged
        .iter()
        .map(|site| finding(&site.path, LABEL, 0, site.gone))
        .chain(functions.iter().map(|function| {
            finding(
                &function.site.file,
                &function.site.text,
                function.site.line,
                function.gone,
            )
        }))
        .collect()
}

/// Today's scope, refused when an `in` selects no test the working tree holds.
fn today(project: &Project) -> Result<Scope, Error> {
    let config = &project.config;
    let values = config.policy(SECTION, KEYS)?;
    let scope = Scope::read(config, project.moves(), SECTION, &values)?;
    let tests = Tests {
        roots: Roots::new(&project.facts().found.test_roots),
        scope,
    };
    if tests.scope.has_in() && !project.tree().files()?.iter().any(|path| tests.holds(path)) {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable file",
            config.file.display()
        )));
    }
    Ok(tests.scope)
}

/// The test roots this run found, as the one `derived:` line and its JSON entry.
pub fn derive(project: &Project) -> Result<Vec<contract::Provenance>, Error> {
    let roots = &project.facts().found.test_roots;
    if roots.is_empty() {
        return Ok(Vec::new());
    }
    let said = contract::Derived::bare(
        SECTION,
        TEST_ROOTS,
        roots.clone().into(),
        roots.join(", "),
        ROOTS_RULE,
    );
    Ok(vec![said.into()])
}

/// Every test function the base holds, with what the working tree says about it. A match is by
/// site first and then by body hash across files, so a test renamed or moved with its body
/// unchanged is held and only a test that was edited as it moved reads as gone. Both trees are
/// read under the scope the base records, so a narrowing loses no test it held. Spec 4.4, 8.2,
/// 16.4.
fn tests_of(tests: &Tests, at: &Context, prior: &Prior) -> Result<Measured, Error> {
    let config = &at.project.config;
    let after = walked(tests, at.project.tree())?;
    let before = walked(tests, prior.tree())?.tests;
    let found = still_there(&before, &after.tests);
    let refused: BTreeSet<&str> = after
        .unparsed
        .iter()
        .map(|file| file.file.as_str())
        .collect();
    Ok(Measured {
        functions: before
            .iter()
            .enumerate()
            .filter(|(_, site)| !refused.contains(site.file.as_str()))
            .map(|(at, site)| Function {
                gone: !found[at],
                file_went: !config.root().join(&site.file).is_file(),
                site: site.clone(),
            })
            .collect(),
        unparsed: after.unparsed,
    })
}

/// Which of the base's test functions the working tree still holds: by site first, then by
/// body hash across files, and one to one on both passes. Spec 4.4, 16.4.
fn still_there(before: &[Test], after: &[Test]) -> Vec<bool> {
    let mut taken = vec![false; after.len()];
    let mut found = vec![false; before.len()];
    for (at, site) in before.iter().enumerate() {
        found[at] = claimed(after, &mut taken, |other| {
            other.file == site.file && other.text == site.text
        });
    }
    for (at, site) in before.iter().enumerate() {
        if !found[at] {
            found[at] = claimed(after, &mut taken, |other| other.body == site.body);
        }
    }
    found
}

/// Whether the working tree holds a test function this rule names that no earlier site has
/// already claimed. Pairing is one to one, so two tests that share a site and lose one of the
/// pair leave that one gone. Spec 4.4.
fn claimed(after: &[Test], taken: &mut [bool], matches: impl Fn(&Test) -> bool) -> bool {
    let found = after
        .iter()
        .enumerate()
        .find(|(at, other)| !taken[*at] && matches(other));
    match found {
        Some((at, _)) => {
            taken[at] = true;
            true
        }
        None => false,
    }
}

/// Every test function one tree holds, by the convention table of 8.2, off the tree's one file
/// list.
fn walked(tests: &Tests, tree: &Tree) -> Result<Walk, Error> {
    let extensions = syntax::extensions(&[]);
    let mut walk = Walk {
        tests: Vec::new(),
        unparsed: Vec::new(),
    };
    for file in tree.files()?.iter().filter(|file| {
        tests.holds(file) && extensions.iter().any(|extension| file.ends_with(extension))
    }) {
        let path = tree.root().join(file);
        let bytes = std::fs::read(&path).map_err(|why| Error::unreadable(&path, why))?;
        let source = String::from_utf8_lossy(&bytes);
        walk.tests
            .extend(convention::tests(file, &source, &mut walk.unparsed)?);
    }
    walk.tests
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(walk)
}

/// What this gate discovered: every test file the base holds, which is also
/// the file scope the function identity reads. A deleted test whose subject went with it is
/// found and not measured, because it is a NOTE and not a site the gate judges. A file no
/// grammar read is unreadable and not measured, because no function in it was seen. Spec 8.6.
fn covered(judged: &[Site], paired: &[Site], unparsed: &[Unparsed], at: &Context) -> Coverage {
    let only = at.only;
    let paths =
        |sites: &[Site]| -> Vec<String> { sites.iter().map(|site| site.path.clone()).collect() };
    let refused: Vec<String> = unparsed.iter().map(|file| file.file.clone()).collect();
    let mut read = paths(judged);
    read.retain(|file| !refused.contains(file));
    let measured = coverage::scoped(&read, only);
    let unreadable = coverage::scoped(&refused, only);
    Coverage {
        found: measured + unreadable + coverage::scoped(&paths(paired), only),
        measured,
        not_measured: 0,
        excluded: 0,
        unreadable,
    }
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[MISSING],
        unit: "test site(s)",
        condition: "where the base holds a test site the working tree no longer has",
        fix_advice: Remedy::Fixed(REMEDY),
        ceiling: None,
        format_metrics: show,
        nested: None,
    }
}

fn show(values: &Values) -> String {
    format!(
        "{MISSING} {}",
        values.get(MISSING).and_then(Value::as_u64).unwrap_or(0)
    )
}

fn finding(path: &str, text: &str, line: u64, gone: bool) -> Finding {
    let mut values = Values::new();
    values.insert(MISSING.into(), u64::from(gone).into());
    Finding {
        file: path.to_string(),
        line,
        text: text.to_string(),
        values,
        body: None,
    }
}

fn gone(site: &Finding) -> bool {
    site.values.get(MISSING).and_then(Value::as_u64) == Some(1)
}

/// The base's entry for every finding, and the deleted tests this run lets through: outside the
/// hook every one, and in the hook the ones a stop's block already asked the agent about. The
/// base entry of a deletion let through carries `missing: 1`, so the one judge holds it and an
/// accepted entry naming it still takes the match. Spec 8.2, 16.4.
fn let_through(now: &[Finding], at: &Context, root: &Path) -> (Vec<Finding>, Vec<Finding>) {
    let asked = match at.hook() {
        true => turn::asked(root),
        false => Vec::new(),
    };
    let through = |site: &Finding| {
        gone(site) && (!at.hook() || asked.contains(&ratchet::identity(at.gate, site)))
    };
    let before = now
        .iter()
        .map(|site| finding(&site.file, &site.text, site.line, through(site)))
        .collect();
    let went = now
        .iter()
        .filter(|site| through(site) && in_scope(at, &site.file))
        .map(|site| finding(&site.file, &site.text, site.line, true))
        .collect();
    (before, went)
}

fn in_scope(at: &Context, file: &str) -> bool {
    at.only
        .is_none_or(|only| only.iter().any(|named| named == file))
}

/// The deleted tests this run lets through, which are a review item at `klin check` and a note
/// at the Stop, never a finding. Removing a test is ordinary work, and klin cannot tell why a
/// test went, so outside the hook a deletion is left to the reviewer who reads the diff.
/// Spec 9.2.
fn deleted(went: &[Finding], at: &Context, out: &mut Sink) {
    if went.is_empty() {
        return;
    }
    out.tell(Listed::TestsDeleted {
        went: went.iter().map(told).collect(),
        caller: at.caller,
    });
}

/// A finding as the site a report lists.
fn told(site: &Finding) -> contract::Located {
    contract::Located {
        file: site.file.clone(),
        line: site.line,
        text: site.text.clone(),
    }
}

/// A deleted test function whose file went in the same window, which is a NOTE and not a
/// finding. #45 judges the file, and for a function the file is the whole subject. Spec 8.2.
fn orphaned(orphans: &[Function], out: &mut Sink) {
    if orphans.is_empty() {
        return;
    }
    out.tell(Listed::TestFunctionsOrphaned(
        orphans
            .iter()
            .map(|function| contract::Located {
                file: function.site.file.clone(),
                line: function.site.line,
                text: function.site.text.clone(),
            })
            .collect(),
    ));
}

/// A deleted test whose subject went in the same window, which is a NOTE and not a finding.
/// Spec 8.2, 16.4.
fn noted(paired: &[Site], out: &mut Sink) {
    if paired.is_empty() {
        return;
    }
    out.tell(Listed::TestFilesPaired {
        files: paired
            .iter()
            .map(|site| contract::Site {
                file: site.path.clone(),
                text: site.subject.clone().unwrap_or_default(),
            })
            .collect(),
        rule: RULE,
    });
}

/// Every test file the base holds, each with what the working tree says about it. This is the
/// one measure of `after` that reads `before`. Spec 16.4.
fn sites(tests: &Tests, listed: &BTreeSet<String>, root: &Path) -> Vec<Site> {
    let mut wanted: Vec<&String> = listed.iter().filter(|path| tests.holds(path)).collect();
    wanted.sort();
    wanted
        .into_iter()
        .map(|path| {
            let gone = !root.join(path).is_file();
            Site {
                path: path.clone(),
                gone,
                subject: gone.then(|| subject(path, listed, root)).flatten(),
            }
        })
        .collect()
}

/// The file the base holds at the test's path with the affixes stripped, gone from the working
/// tree too. `None` when no candidate resolves, and then the test is judged normally.
fn subject(path: &str, listed: &BTreeSet<String>, root: &Path) -> Option<String> {
    candidates(path)
        .into_iter()
        .find(|candidate| listed.contains(candidate) && !root.join(candidate).is_file())
}

/// Every path the affix table names as a subject: the directory with one test segment removed,
/// the basename with one affix removed, and both together.
fn candidates(path: &str) -> Vec<String> {
    let (directory, name) = match path.rsplit_once('/') {
        Some((directory, name)) => (directory.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    };
    let mut directories = vec![directory.clone()];
    directories.extend(without_a_test_dir(&directory));
    let mut names = vec![name.clone()];
    names.extend(without_an_affix(&name));
    let mut out = Vec::new();
    for directory in &directories {
        for name in &names {
            let candidate = match directory.is_empty() {
                true => name.clone(),
                false => format!("{directory}/{name}"),
            };
            if candidate != path {
                out.push(candidate);
            }
        }
    }
    out
}

fn without_a_test_dir(directory: &str) -> Vec<String> {
    let segments: Vec<&str> = match directory.is_empty() {
        true => Vec::new(),
        false => directory.split('/').collect(),
    };
    segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| TEST_DIRS.contains(*segment))
        .map(|(at, _)| {
            let mut kept = segments.clone();
            kept.remove(at);
            kept.join("/")
        })
        .collect()
}

fn without_an_affix(name: &str) -> Vec<String> {
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (name, String::new()),
    };
    let prefixes = TEST_PREFIXES
        .iter()
        .filter_map(|prefix| stem.strip_prefix(prefix));
    let suffixes = TEST_SUFFIXES
        .iter()
        .filter_map(|suffix| stem.strip_suffix(suffix));
    prefixes
        .chain(suffixes)
        .filter(|stripped| !stripped.is_empty())
        .map(|stripped| format!("{stripped}{extension}"))
        .collect()
}

/// The base commit the runner chose, or the one this gate chooses for itself.
/// The base tree's file list, read out of git so the file level needs no base worktree. A
/// listing git refuses is an error: an empty base holds no test file and reports green.
fn at_the_base(root: &Path, commit: &str) -> Result<BTreeSet<String>, Error> {
    let listed = Repo::at(root)
        .text(&["ls-tree", "-r", "--name-only", commit, "--", "."])
        .ok_or_else(|| {
            Error(format!(
                "the base commit {} could not be listed under {} — fetch history, or give CI \
                 the full clone",
                &commit[..7.min(commit.len())],
                root.display()
            ))
        })?;
    Ok(listed
        .lines()
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}
