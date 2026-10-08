//! What a check is, told once. A check is handed an immutable `Context` — who called it, which
//! gate it judges under, what base it compares against, what scope it may look at — and it
//! writes to an explicit `Sink`: the typed `Told` items the renderers turn into the report a
//! person reads and into the findings, notes and derived entries of the 11.2 object, and the
//! `Records` of what the run cost and covered, which the runner adds to that object. Nothing
//! about a run reaches a check any other way, so the runner keeps no side channel into it and a
//! check keeps no state of its own. This module names no check, so every check and the catalogue that
//! registers them depend on it one way.

use std::path::Path;

use serde_json::{Map, Value};

use crate::base::{self, Prior, Window};
use crate::ceiling::Ceiling;
use crate::changed::Change;
use crate::config::Config;
use crate::coverage::{Coverage, Left};
use crate::error::Error;
use crate::files::Form;
use crate::key::Key;
use crate::measurement::{self, Unchanged};
use crate::project::Project;
use crate::record::Values;
use crate::syntax::structural::{ExtractionCost, NameCost, footprint::Footprint};

use crate::{modules, surface};

/// The outcome of a file no grammar reads. The hook counts these to report the holes a
/// person must close, and nothing else in a run turns on it.
pub const UNPARSED: &str = "unparsed";

/// The outcome of a test the base holds that went in the window, which fails nothing. A stop the
/// hook lets end hands it to a person. Spec 8.2.
pub const DELETED: &str = "deleted";

/// The review item kind of a pinned policy path that selects no file of the working tree.
/// Spec 7.3.
pub const MOVED_PIN: &str = "moved-pin";

/// The note kind of what choosing the base of a `klin check` window found. Spec 6.5.
pub const WINDOW: &str = "window";

/// The review item kind of a deleted test at `klin check`. Spec 9.2, 11.7.
pub const DELETED_TEST: &str = "deleted-test";

/// The review item kind of an accepted entry that matched nothing, which only a person acts on.
/// Spec 7.6.
pub const UNMATCHED_ACCEPTED: &str = "unmatched-accepted";

/// The outcome of a file `before` measured and `after` did not, which a run records so a report
/// never reads a window it stopped measuring as a whole one. Spec 8.6.
pub const LOST: &str = "lost";

/// The outcome of a parser-readable file for which no semantic adapter exists. It is a hole in
/// a structural gate, not a green measurement. Spec 8.4, 8.6.
pub const NOT_MEASURED: &str = "not-measured";

/// The outcome of a derived ceiling whose recorded scope fell back to the whole repository or
/// differs from today's. The hook tells it, so a scope lag is never silent. Spec 5.4, ADR 0039.
pub const DERIVATION: &str = "derivation";

/// The outcome of a build whose command the shell could not find. The tool is absent, so the
/// tree is unmeasured rather than failing: the gates judge the source and the hook tells the
/// note, because the one action left is an install. ADR 0048.
pub const UNBUILT: &str = "unbuilt";

/// The outcome of a dependency form a module resolver supports and could not resolve. A green
/// layering run must not imply a resolution klin did not make. Spec 8.2.1, 8.6.
pub const UNRESOLVED: &str = "unresolved";

/// The source-content work a file-local gate performed over its current and base trees. The
/// runner exposes this beside structural `facts` so performance rows can prove a changed run
/// did not read unchanged source.
#[derive(Default, Clone, Copy)]
pub struct ContentCost {
    pub reads: usize,
    pub parses: usize,
}

impl std::ops::Add for ContentCost {
    type Output = ContentCost;

    fn add(self, other: ContentCost) -> ContentCost {
        ContentCost {
            reads: self.reads + other.reads,
            parses: self.parses + other.parses,
        }
    }
}

/// What one gate's run cost and covered, which the runner puts on the gate's row of spec 11.2,
/// and why it erred. A check a person runs by hand has none, and records nothing. The findings,
/// notes and derived entries are not here: the runner renders them from the gate's `Told`.
#[derive(Default)]
pub struct Records {
    /// Why a gate that is exit 2 failed where it names no site. The runner records them as one
    /// `error` finding, and only for a gate that recorded no other finding.
    pub errors: Vec<String>,
    /// What scope the gate measured, which every check records once. Spec 11.2.
    pub coverage: Option<Value>,
    /// The findings the ratchet passed, which the runner puts on the gate's row. `None` for a
    /// gate that never got that far. Spec 11.2.
    pub held: Option<u64>,
    /// How many of `held` a person-authored accepted entry passed rather than a base site, which
    /// is the one number the OK line's qualifier turns on. `None` for a gate that never got that
    /// far, and 0 for a ratcheting gate no accepted entry matched. Spec 11.2.
    pub accepted: Option<u64>,
    /// The structural facts the gate read over both trees: extracted by it, or shared from an
    /// earlier gate of the run. `None` for a gate that reads none. Spec 11.2.
    pub facts: Option<ExtractionCost>,
    /// The source contents a file-local gate read and parsed over both trees. `None` for a gate
    /// that records no content work.
    pub work: Option<ContentCost>,
    /// The declaration states `dead-symbols` built over both trees. `None` for a gate that
    /// builds none. Spec 11.2.
    pub states: Option<u64>,
    /// The name evidence `dead-symbols` and `reachability` built over both trees. `None` for a
    /// gate that resolves no names. Spec 11.2.
    pub names: Option<NameCost>,
    /// The parts of laying the whole base out, on the row of the gate that laid it out. `None`
    /// for every other gate. Spec 11.2.
    pub layout: Option<base::Layout>,
    /// What the facts `dead-symbols` held over both trees cost in population and bytes. `None`
    /// for a gate that records none. Spec 11.2.
    pub footprint: Option<Footprint>,
    /// The module graphs the gate built over both trees. `None` for a gate that builds none.
    pub graph: Option<modules::GraphCost>,
    /// The public surfaces the gate derived over both trees. `None` for a gate that derives
    /// none. Spec 11.2.
    pub surface: Option<surface::SurfaceCost>,
}

/// Who ran this check: `klin check`, or the stop hook, where a hole the agent cannot fix is a
/// note and not a failure. The runner prints the run's context once for every check. Spec 4.3,
/// 8.2, 11.1.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Caller {
    Gate,
    Hook,
}

/// What a check needs of the base: nothing, the commit the window names, or that commit laid
/// out as a tree beside the working one. A check that needs the commit or the tree is also the
/// kind `--strict` reaches, because it has a comparison or an accepted list to judge. Spec
/// 4.6, 10.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Needs {
    Nothing,
    TheCommit,
    TheTree,
}

impl Needs {
    /// Whether the run resolves the base commit for this check, which is also whether
    /// `--strict` reaches it. Spec 4.6, 10.
    pub fn the_commit(self) -> bool {
        self >= Needs::TheCommit
    }

    /// Whether the run lays the base commit out as a tree for this check.
    pub fn the_tree(self) -> bool {
        self == Needs::TheTree
    }
}

/// What absence of a check's section means, which is the one thing a person needs to know
/// about a check before writing its section. Spec 4.6, ADR 0038.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Absence means the check runs over the facts of the tree and derives its own policy.
    Automatic,
    /// Absence means the project has no such policy, so the check does not run.
    Policy,
    /// Absence means no external tool is configured, so the check does not run.
    Integration,
}

/// The execution paths at which the engine runs a capability. The catalogue owns it, and no
/// configuration changes it. Spec 4.3, 6.2.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    StopAndCheck,
    Check,
}

impl Placement {
    pub fn at_stop(self) -> bool {
        self == Placement::StopAndCheck
    }

    pub fn names(self) -> &'static [&'static str] {
        match self {
            Placement::StopAndCheck => &["stop", "check"],
            Placement::Check => &["check"],
        }
    }
}

/// Why a required measurement is not complete. Spec 7.2.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    ToolError,
    NothingMeasured,
    Unsupported,
    /// A base equal to HEAD whose local source may hide unpushed commits. Spec 6.5.
    ComparisonUnproven,
    /// No shipped capability has a work bound yet, so only a test reaches it. Spec 7.2.
    #[cfg_attr(not(test), allow(dead_code))]
    WorkLimit,
}

impl Reason {
    pub fn name(self) -> &'static str {
        match self {
            Reason::ToolError => "tool-error",
            Reason::NothingMeasured => "nothing-measured",
            Reason::Unsupported => "unsupported",
            Reason::ComparisonUnproven => "comparison-unproven",
            Reason::WorkLimit => "work-limit",
        }
    }
}

/// One explicit reason why a required measurement is not complete, with the detail a consumer
/// branches on and the words a person reads. Spec 7.2.
pub struct Incomplete {
    pub reason: Reason,
    pub detail: Option<&'static str>,
    pub text: String,
}

/// Everything one check is told, and nothing it writes. Borrowed for the length of the call, so
/// a check cannot keep any of it and cannot change it.
pub struct Context<'a> {
    /// The name of the gate being run, which the accepted list names.
    pub gate: &'a str,
    /// The one configuration the run loaded, and the facts of the working tree, read once for
    /// every check the run selects. ADR 0038.
    pub project: &'a Project,
    /// The base commit, already laid out as a tree by the runner.
    pub prior: Option<&'a Prior>,
    /// The base commit the runner chose, for a gate that reads the base tree out of git.
    pub base: Option<&'a str>,
    /// The files a scoped run judges, and `None` for a run that judges everything.
    pub only: Option<&'a [String]>,
    /// The authoritative before-to-after changes of a changed run, separate from its judgement
    /// scope. Direct checks and unscoped runs have no changed-run context.
    pub changes: Option<&'a [Change]>,
    pub caller: Caller,
}

impl Context<'_> {
    /// Whether the Stop hook runs this gate, so a hole the agent cannot fix is a note, not a
    /// failure.
    pub fn hook(&self) -> bool {
        self.caller == Caller::Hook
    }

    /// The configuration the run loaded, which every check reads its section from.
    pub fn config(&self) -> &Config {
        &self.project.config
    }

    /// The same run with another judgement scope, for a check that derives a narrower or wider
    /// effective scope from evidence the runner does not read. Nothing else moves.
    pub fn scoped<'b>(&self, only: Option<&'b [String]>) -> Context<'b>
    where
        Self: 'b,
    {
        Context {
            only,
            gate: self.gate,
            project: self.project,
            prior: self.prior,
            base: self.base,
            changes: self.changes,
            caller: self.caller,
        }
    }
}

/// Where a check writes: what it found, in the order a report says it, and the records the
/// runner keeps. A check writes no report text: `render` does. Spec 3.1, 6.1.
pub struct Sink<'a> {
    pub told: &'a mut Vec<Told>,
    pub records: &'a mut Records,
}

/// What a gate judged, for the `OK:` line: its counts in the check's own terms, and the coverage.
/// The renderer words both. Spec 8.6.
pub struct Line {
    pub judged: Judged,
    pub coverage: Option<Coverage>,
}

impl Line {
    pub fn new(judged: impl Into<Judged>, coverage: Option<Coverage>) -> Line {
        Line {
            judged: judged.into(),
            coverage,
        }
    }
}

/// The counts a gate's `OK:` line states, one variant per gate's terms.
pub enum Judged {
    Counted(Counted),
    Measured(Measured),
}

/// The counts of a gate that judges what it finds one by one.
pub enum Counted {
    Documents(usize),
    Conventions(usize),
    Convention {
        name: String,
        sites: usize,
    },
    Citations(usize),
    Dependencies {
        judged: usize,
        manifests: usize,
    },
    Tests {
        held: usize,
        gone: usize,
    },
    Markers {
        sites: usize,
        unit: &'static str,
        skipped: u64,
    },
}

/// The counts of a gate that measures structure over a tree.
pub enum Measured {
    DeadSymbols {
        judged: usize,
        dead: usize,
    },
    Reachability {
        judged: usize,
        unreached: usize,
        unjudged: usize,
    },
    Complexity(Complexity),
    Layering(Layering),
    PublicApi(PublicApi),
    Sarif {
        judged: u64,
        held: u64,
        differential: bool,
    },
}

/// The functions `complexity` judged, the dated steps of the ceilings in force, and the test
/// functions it did not judge on length with the files among them a window added or renamed.
pub struct Complexity {
    pub judged: usize,
    pub over: usize,
    pub steps: Vec<(&'static str, Ceiling)>,
    pub unjudged: usize,
    pub arrived: Vec<String>,
}

/// The dependency sites `layering` judged, and how the module graph attached the files.
#[derive(Default)]
pub struct Layering {
    pub sites: usize,
    pub forbidden: usize,
    pub cyclic: usize,
    pub attached: usize,
    pub by_manifest: usize,
    pub by_convention: usize,
    pub unattached: usize,
    pub external: usize,
}

/// The external items `public-api` judged, and the surfaces it discovered.
pub struct PublicApi {
    pub items: usize,
    pub surfaces: usize,
    pub measured: usize,
    pub opaque: usize,
    pub rust: usize,
    pub typescript: usize,
    pub inapplicable: usize,
}

impl From<Counted> for Judged {
    fn from(counted: Counted) -> Judged {
        Judged::Counted(counted)
    }
}

impl From<Measured> for Judged {
    fn from(measured: Measured) -> Judged {
        Judged::Measured(measured)
    }
}

/// The kind of form a gate supports and could not resolve. Spec 8.2.1, 8.6.
#[derive(Clone, Copy)]
pub enum Unresolvable {
    Dependency,
    PublicSurface,
}

/// One file a list in the report names, with the text the list says of it.
pub struct Site {
    pub file: String,
    pub text: String,
}

/// One site at a line of a file, with its text.
pub struct Located {
    pub file: String,
    pub line: u64,
    pub text: String,
}

/// What a ratchet failure was compared against: nothing, the accepted entry for a file, or the
/// base site at a file and line. Spec 8.6.
pub enum Matched {
    Nothing,
    Accepted(Entry),
    Base(Entry),
}

/// A base site or accepted entry, with the values it holds apart from the keys of its site.
pub struct Entry {
    pub file: String,
    pub line: Option<u64>,
    pub text: String,
    pub values: Values,
}

/// One finding the ratchet failed: its site and identity, its values and the check's own words
/// for them, and what the ratchet held against it. Spec 8.6, 11.2.
pub struct Failed {
    pub id: String,
    pub file: String,
    pub line: u64,
    pub values: Values,
    pub shown: String,
    pub was: Option<Was>,
    pub text: String,
    pub fix_advice: String,
    pub matched: Matched,
    pub ceiling: Option<String>,
    /// The new finding whose group this one prints inside, for a gate whose text report groups
    /// what one change took away. Presentation only.
    pub lead: Option<usize>,
}

/// An accepted entry that matched nothing this run: the entry, the check's words for its values,
/// and the row it names that klin retired.
pub struct Unmatched {
    pub entry: Entry,
    pub shown: String,
    pub retired: Option<String>,
}

/// What the base or accepted entry held for a finding that got worse: the check's words for its
/// values, and the file it held them at where that is not the finding's own.
#[derive(Clone)]
pub struct Was {
    pub shown: String,
    pub at: Option<String>,
}

/// How many of a gate's passing findings an accepted entry held, and how many a base site held.
/// None of either claims nothing. Spec 8.6.
#[derive(Default, Clone, Copy)]
pub struct Held {
    pub accepted: usize,
    pub base: usize,
}

/// Where a document stands against its ceiling. Spec 8.1.
pub enum Standing {
    Under,
    Near(u64),
    Held(u64),
    Over {
        /// The document's identity, a site with no line or text. Spec 11.7.
        id: String,
        condition: &'static str,
        fix_advice: &'static str,
    },
}

/// One item of a gate's semantic result, in the order the report says it. Spec 3.1, 6.1.
pub enum Told {
    Judged {
        line: Line,
        held: Held,
    },
    Document {
        name: String,
        words: u64,
        ceiling: Ceiling,
        standing: Standing,
    },
    Provenance(Provenance),
    Plain(Plain),
    Hole(Hole),
    Listed(Listed),
    Ratchet(Ratchet),
    Incomplete(Incomplete),
}

/// One value of a check's policy and where it came from: pinned by a person, or derived by a
/// rule. Spec 4.3, 11.2.
pub enum Provenance {
    Pinned {
        section: &'static str,
        key: String,
        shown: String,
    },
    Derived(Derived),
}

/// A value a check derived, and the rule that derived it. Spec 11.2.
pub struct Derived {
    pub section: &'static str,
    pub key: Option<String>,
    pub value: Value,
    /// The value as the `derived:` line words it.
    pub shown: String,
    pub rule: String,
    pub wording: Wording,
}

/// How a `derived:` line names its value and gives its rule.
pub enum Wording {
    /// Under its section and key, with the rule after a comma.
    Keyed,
    /// Under its key alone, a key that names itself, with the rule after a comma.
    Bare,
    /// Under its section and key, with the rule in parentheses and then the scope the rule read,
    /// which the JSON rule carries joined.
    Sampled(String),
}

/// A NOTE that fails nothing: what it is about, the file it names, and its text.
pub struct Note {
    pub outcome: &'static str,
    pub file: String,
    pub text: String,
}

/// A line the report says as it is, under its own word.
pub enum Plain {
    Note(Note),
    Remedy(String),
    PathMissing(String),
    Error(String),
}

/// What the change did to a file klin cannot measure. Spec 7.2.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// The base measured the file and the change made it unmeasurable: a FAIL.
    Lost,
    /// The change made it unmeasurable with no clear agent cause: a review item.
    Opened,
    /// klin's own limit, which the change did not open: a coverage note.
    Limit,
}

/// Why klin cannot measure a file, which names the finding, review item or coverage note of
/// its class. Spec 7.2.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Parse,
    LineCeiling,
    Manifest,
    Form,
    Unreadable,
    NotText,
    ResourceLimit,
    Filtered,
    LeftScope,
}

impl Cause {
    pub fn name(self) -> &'static str {
        const NAMES: [&str; 9] = [
            "parse",
            "line-ceiling",
            "manifest",
            "form",
            "unreadable",
            "not-text",
            "resource-limit",
            "filtered",
            "left-scope",
        ];
        NAMES[self as usize]
    }
}

/// What a gate could not measure. The gate names the file and what it saw, and the runner sorts
/// each file once for the run against the base. Spec 7.2.
pub enum Hole {
    /// A file `before` measured and `after` does not, though the tree still holds it, and why.
    Lost { file: String, why: Left },
    /// The files in the gate's scope no grammar read, each with its grammar's name.
    Unparsed(Vec<Site>),
    /// A file the gate measured that the working tree's `.gitattributes` give a form.
    Formed {
        file: String,
        form: Form,
        measured: bool,
    },
    /// A manifest klin could not parse, which the gate already classed against the base.
    Manifest { site: Site, class: Class },
    /// Forms the gate supports and could not resolve, `opened` where the base held none of them.
    Unresolved {
        opened: bool,
        kind: Unresolvable,
        forms: Vec<(Located, String)>,
    },
}

/// Sites a gate lists that fail nothing: dead symbols a direct run reports, and the tests a
/// window let through. Spec 8.2.
pub enum Listed {
    DeadSymbols(Vec<(Located, String)>),
    /// The deleted tests a window let through: a review item at `klin check`, and a note at the
    /// Stop. Spec 9.2.
    TestsDeleted {
        went: Vec<Located>,
        caller: Caller,
    },
    TestFunctionsOrphaned(Vec<Located>),
    TestFilesPaired {
        files: Vec<Site>,
        rule: &'static str,
    },
    /// Sites the base already held, which fail nothing because the change opened none.
    Held(HeldAtBase),
    /// The packages and targets `public-api` derived no surface from.
    NoSurface(Vec<Site>),
}

/// What the base already held, of a gate that names it.
pub enum HeldAtBase {
    DeadSymbols(Vec<Located>),
    Edges(Vec<Site>),
    Unreached(Vec<String>),
}

/// What the ratchet's comparison came to beyond the `OK:` line. Spec 4.4, 8.6.
pub enum Ratchet {
    New {
        unit: String,
        condition: String,
        held: usize,
        failed: Vec<Failed>,
    },
    Worse {
        unit: String,
        condition: String,
        failed: Vec<Failed>,
    },
    /// The accepted entries that matched nothing: a review item at `klin check`, and a note at
    /// the Stop. Never a failure. Spec 7.6.
    AcceptedUnmatched {
        entries: Vec<Unmatched>,
        caller: Caller,
    },
}

impl Told {
    /// The `OK:` line of a gate that compares nothing, so nothing held its findings.
    pub fn judged(line: Line) -> Told {
        Told::Judged {
            line,
            held: Held::default(),
        }
    }
}

impl From<Provenance> for Told {
    fn from(said: Provenance) -> Told {
        Told::Provenance(said)
    }
}

impl From<Plain> for Told {
    fn from(said: Plain) -> Told {
        Told::Plain(said)
    }
}

impl From<Hole> for Told {
    fn from(hole: Hole) -> Told {
        Told::Hole(hole)
    }
}

impl From<Incomplete> for Told {
    fn from(hole: Incomplete) -> Told {
        Told::Incomplete(hole)
    }
}

impl From<Listed> for Told {
    fn from(listed: Listed) -> Told {
        Told::Listed(listed)
    }
}

impl From<Ratchet> for Told {
    fn from(said: Ratchet) -> Told {
        Told::Ratchet(said)
    }
}

impl<'a> Sink<'a> {
    pub fn tell(&mut self, told: impl Into<Told>) {
        self.told.push(told.into());
    }

    pub fn record(&mut self, add: impl FnOnce(&mut Records)) {
        add(self.records);
    }

    /// Why this gate is exit 2 where the failure names no site. See `Records::errors`.
    pub fn error(&mut self, text: String) {
        self.records.errors.push(text);
    }

    /// A NOTE that fails nothing, about `file`, recorded for `--json` under `outcome`.
    pub fn note(&mut self, outcome: &'static str, file: String, text: String) {
        self.tell(Plain::Note(Note {
            outcome,
            file,
            text,
        }));
    }
}

/// The base commit a gate judges against: the one the runner chose, or the one this gate
/// chooses for itself and names once in the report. Spec 6.1.
pub fn base_commit(root: &Path, at: &Context) -> Result<String, Error> {
    match at.base {
        Some(commit) => Ok(commit.to_string()),
        None => Ok(announced(root)?.before),
    }
}

/// The base a gate the runner did not lay out chooses for itself.
pub fn announced(root: &Path) -> Result<Window, Error> {
    base::choose(root)
}

/// The base laid out whole for this run: the runner's own when it laid the whole base out, which
/// a changed run never does, and otherwise the run's one checkout. Spec 8.4, ADR 0038.
pub fn whole_base<'a>(at: &Context<'a>, commit: &str) -> Result<&'a Prior, Error> {
    match at.prior.filter(|_| at.changes.is_none()) {
        Some(prior) => Ok(prior),
        None => at.project.whole_base(commit, shared(at)),
    }
}

/// The base's view of the files this run leaves alone, which only a changed run that is not
/// strict shares. Spec 8.4.
pub fn unchanged_base<'a>(
    at: &Context<'a>,
    prior: &'a Prior,
    commit: &str,
) -> Result<Option<Unchanged<'a>>, Error> {
    measurement::unchanged(at.project, shared(at), prior, commit)
}

fn shared<'a>(at: &Context<'a>) -> Option<&'a [Change]> {
    at.changes
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own_base(at: &Context) -> Result<Prior, Error> {
    let base = announced(at.project.root())?;
    base::materialize(at.project, &base.before, None)
}

impl Sink<'_> {
    /// What every `OK:` line adds after what the gate judged, recorded for `--json` on the way
    /// past so one call per check carries both. Spec 8.6.
    pub fn covered(&mut self, coverage: &Coverage) -> Option<Coverage> {
        self.record(|records| records.coverage = Some(coverage_record(coverage)));
        Some(*coverage)
    }
}

fn coverage_record(coverage: &Coverage) -> Value {
    let mut out = Map::new();
    out.insert("found".into(), coverage.found.into());
    out.insert("measured".into(), coverage.measured.into());
    out.insert("not_measured".into(), coverage.not_measured.into());
    out.insert("excluded".into(), coverage.excluded.into());
    out.insert("unreadable".into(), coverage.unreadable.into());
    Value::Object(out)
}

pub type Run = fn(&Context<'_>, &mut Sink<'_>) -> Result<u8, Error>;

/// A check's own explanation of its derived policy, of every entry or of the one a person names.
pub type Explain = fn(&Project, Option<&str>) -> Result<Vec<String>, Error>;

/// One `derived:` line of the runner's own build and the `{section, key, value, rule}` entry
/// beside it, built together so the two cannot say different things. Spec 11.2.
pub type Said = (String, Option<Value>);

/// The `{section, key, value, rule}` entry `--json` prints beside a `derived:` line. Spec 11.2.
pub fn derived_entry(section: &str, key: Option<&str>, value: Value, rule: &str) -> Value {
    serde_json::json!({ "section": section, "key": key, "value": value, "rule": rule })
}

impl Derived {
    /// A derived value under its section and key, with the rule after a comma.
    pub fn keyed(
        section: &'static str,
        key: Option<&str>,
        value: Value,
        shown: String,
        rule: &str,
    ) -> Derived {
        Derived {
            section,
            key: key.map(str::to_string),
            value,
            shown,
            rule: rule.to_string(),
            wording: Wording::Keyed,
        }
    }

    /// A derived value under a key that names itself, with the rule after a comma.
    pub fn bare(
        section: &'static str,
        key: &str,
        value: Value,
        shown: String,
        rule: &str,
    ) -> Derived {
        Derived {
            wording: Wording::Bare,
            ..Derived::keyed(section, Some(key), value, shown, rule)
        }
    }

    /// A value sampled under its section and key, with the rule in parentheses and then the
    /// scope the sample read.
    pub fn sampled(
        (section, key): (&'static str, &str),
        value: Value,
        shown: String,
        (rule, recorded): (&str, String),
    ) -> Derived {
        Derived {
            wording: Wording::Sampled(recorded),
            ..Derived::keyed(section, Some(key), value, shown, rule)
        }
    }
}

impl From<Derived> for Provenance {
    fn from(derived: Derived) -> Provenance {
        Provenance::Derived(derived)
    }
}

impl From<Derived> for Told {
    fn from(derived: Derived) -> Told {
        Told::Provenance(derived.into())
    }
}

/// The key every entry of a named section carries, whichever check reads the section.
pub const NAMED: Key = Key {
    name: "name",
    holds: "the gate's own name, which `klin check` takes",
    required: true,
    rule: None,
    default: "",
    shape: crate::key::Shape::String,
};

/// The entries of a section a person writes entry by entry, each with the name its gate takes.
/// Such a section is a list, and an entry with no `name` is a config error naming the key,
/// because nothing in a tree says which tool the entry runs. The check that reads one entry
/// reads its own list through this, so a gate's name is the name the check judges under.
/// Spec 8.3.
pub fn named_entries(config: &Config, section: &str) -> Result<Vec<(String, Value)>, Error> {
    let held = config.required(section)?;
    let listed = held.as_array().ok_or_else(|| {
        Error(format!(
            "{}: \"{section}\" is a list of entries, each its own gate under its own \"name\"",
            config.file.display()
        ))
    })?;
    listed
        .iter()
        .map(|entry| {
            let name = entry
                .get(NAMED.name)
                .and_then(Value::as_str)
                .ok_or_else(|| config.missing(section, NAMED.name))?;
            Ok((name.to_string(), entry.clone()))
        })
        .collect()
}
