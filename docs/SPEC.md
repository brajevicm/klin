# klin Specification

Status: Draft v0, 2026-09-08

This specification, accepted ADRs and klin's CLI tests define klin's behaviour.
ADR 0025 retires the external behavioural reference obligation. Gaps and
conflicts are resolved against klin's requirements and recorded here and in
CLI tests.

Purpose: Define a tool that gates an agentic coding harness deterministically,
with at most one committed configuration file, no baseline that a person
maintains, green on the day it arrives, and tighter under a schedule a
person pins once.

The structure follows the Symphony service specification. This draft is not
bound by the ADRs under `docs/adr/`. Where it reverses one, it says so and
gives the reason, so each reversal can be accepted or refused on its own.
Section 0 lists them.

## Normative Language

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY
and OPTIONAL are to be read as RFC 2119 describes them.

Implementation-defined means the behavior is part of the contract, but this
document does not prescribe one policy. An implementation MUST document the
policy it chose.

The vocabulary of `CONTEXT.md` applies. This document adds two terms, Window
and Derived, and asks `CONTEXT.md` to take them.

## 0. Decisions this draft reverses

| ADR | What it decided | What this draft does instead | Why |
|---|---|---|---|
| 0005 | A missing key is an error, never a default. `init` writes every section. | A missing key is derived from the base tree at run time and printed. `init` only pins a derived value into the file. | The goal is a tool a person manages nothing for. A derived value that the run prints is not the silent default ADR 0005 feared. Section 5. |
| 0009, 0013 | The hook compares against the merge-base with the default branch. | The hook compares against the turn stamp. `klin gate` and CI keep the merge-base. | The merge-base is the wrong window for a turn. It is empty after a commit on the default branch, it grows with the branch, and ADR 0013 exists only to patch that. Section 6. |
| 0010 | Under `--strict` an unaccounted gate is exit 2. | Every applicable gate runs unless its section is `false`. The failure has nothing left to catch. | A gate whose section is derived cannot be unaccounted. Section 10. |
| 0007 | Config keys preserve compatibility for differential tests. | One key vocabulary across sections. The differential test goes. | ADR 0009 already weakened the reason to a preference. `sources` in one section and `roots` in the next is a cost a user pays for a test klin no longer needs. |
| 0003 | A file no grammar reads is exit 2 everywhere. | In the hook it is a NOTE. Outside the hook it stays exit 2. | An agent cannot fix a grammar. Blocking a stop on it costs a turn per stop with no remedy. |
| 0011 | A non-reader naming a guarded path is refused. | It is `ask`. Only a clear write is `deny`. | Four read-only commands were refused in the session that wrote this draft. Section 9.4. ADR 0027 keeps the `ask` and shrinks the guarded set to `klin.json`. |
| 0004 | The host's cap on consecutive blocks bounds the loop. | klin bounds its own blocks. | The cap is not in the host's current documentation. Section 9.3. |
| 0015 | klin's state lives in `.klin/` at the tree root, with one `.gitignore` line. | State lives in the git directory, or under `KLIN_STATE_DIR`. No ignore line. | Per-worktree state has a native home that git never tracks and never lists. Section 7.4. |
| 0002 | The plugin ships no binary. | The plugin ships a `bin/klin` wrapper that fetches the pinned release on first run. | The first reason, a pin beside committed baselines, went with ADR 0009. The second, `bin/` unavailable for organization-distributed plugins, is handled by falling back to PATH. Section 19. |
| 0014, one line | The turn stamp is not guarded, because a report leaves nothing to gain. | The stamp is in the guarded set, and a missing stamp widens the window instead of closing it. | The stamp now holds the verdict that keeps a window open. A stamp that is gone is restored from its ref, or the stop judges the whole branch, so deleting it buys nothing. Sections 6.2 and 9.4. The rest of ADR 0014 stands. ADR 0027 reverses this row again: the stamp left the guarded set with everything but `klin.json`, and 9.4 carries the current rule. |

ADR 0001, 0006, 0008 and 0012 stand as written.

Every row above was accepted on 2026-09-08 and is recorded as an ADR: 0016
for the optional config and derivation, 0017 for the turn stamp as the hook's
base, 0018 for one key vocabulary, 0019 for the state directory, 0020 for the
guard's `ask` decision and the guarded stamps, 0021 for the unreadable file in
the hook, 0022 for klin bounding its own blocks, and 0023 for the plugin
wrapper. Each superseded ADR carries a note pointing forward.

This draft was reviewed on 2026-09-08 (`docs/spec-review-2026-09-08.md`).
Every finding of that review is applied here. The review's fix table maps to
sections 5.1, 5.4, 6.2, 6.5, 6.6, 7.3, 8.3, 9.4, 10 and 14.

A second review on 2026-09-09 found six contract gaps. Four are applied: the
derived ceiling is not monotone (5.4, 6.6, 7.3), an acceptance no longer moves
the stamp (6.2, 16.1), roots are discovered in both trees (3, 5.4), and the
dated ceiling uses one zone (5.5). Two were refused: a `complete` flag that
holds the window open on an unreadable file, because such a file yields no
sites in `before` and its later readable form is judged `new`, so the debt is
not hidden, and report provenance for SARIF, because a working tree has no
revision id. The `run` key in 8.3 answers the freshness problem instead. The
review also added the coverage line (8.6, 11), `inventory` over test
declarations (8.2), and three escapes patterns.

A third review on 2026-09-09, of the draft above, found six more. Five are
applied: the day-one promise is narrowed to the same tree (5.1), stops in one
worktree take a lock (6.5), the stamp ref moved under `refs/worktree/` (6.5),
`inventory` has its own judge (16.4), and ADR 0016 and 0017 are amended to
match 5.4, 6.2 and 6.6. The sixth, a content fingerprint of the working tree
as report provenance, was replaced by deleting the report before `run` (8.3).
The review also added the coverage regression rule (8.6, 10), verification
files to the guard's `ask` list, which ADR 0027 later removed again (9.4),
the empty test body to `stubs` (8.2),
a finding id (11.2), and the paired scenarios of 17. `TODO` and `FIXME` stay
in the `stubs` table because #106 had decided it.

A fourth review on 2026-09-09 read the draft for contradictions and found
five that would have shaped code, a data type or a test. Each resolution was
judged against the goals before it went in. Derived values split into
numbers from the derivation commit and path sets unioned with `after`, and a
site under an undiscovered path is `new` (3, 4.3, 5.4, 7.1, 12). `inventory`
ratchets existence under the one judge, which reverses the own judge of the
third review (4.4, 4.5, 8.2, 16.4). A deleted stamp is restored from its ref
or widens the window to the branch, and the state-directory `deny` goes (6.2,
7.4, 9.4, 14, 16.1). The failure model is keyed on `--hook` and `--strict`,
never on CI, and a config error in the hook never blocks (8.6, 10, 14). The
build stamp counts blocks under a prompt counter in `turn`, and a red verdict
is written before a build block (6.2, 9.2, 9.3, 16.1, 16.3). ADR 0016, 0017,
0020 and 0022 carry the amendments. A re-read after the edits found thirteen
smaller conflicts, most between the pseudocode of 16 and the prose it
illustrates, and they are fixed in place: the stamp rule names the first
session (6.2), a check with no ceiling judges every entry (16.4), `inventory`
receives `before` (16.4), the prompt counter is not written into a deleted
stamp (16.1), the edit tool on the state directory is `ask` (9.4, 15.1), the
build stamp is one record per prompt (16.3), and an unwritable state
directory reports a build failure instead of blocking (14). The same pass
fixed the escapes row count

(8.2), the `sarif` exception in 8.2, the `OK:` line with notes (8.6), the
finding id under a rename (11.2), the report path under `.gitignore` (8.3),
the report age check as the second clock (12), and the per-worktree survey
cache (6.6).

A fifth pass on 2026-09-09 tidied every open ticket against this draft and
settled the readings the tickets had made where the draft was silent. Seven
went into the text: the `Test` and `Tests` affixes (8.2), the empty document
ceiling (5.4), what `init --force` keeps (5.7), `--sarif PATH` (11.3), the
heredoc body as data (9.4), the tools out of scope for `compare` (8.3), and
the name-based resolution rule with its first languages (8.4). One check
left: `test-hygiene` is an escapes `patterns` row over the test roots, and
its schedule was the forced paydown 7.3 refuses (8.4).

An independence audit on 2026-09-10 (#127) found that 7.1 and the CLI tests
disagreed on how identical sites match. The tests describe the behaviour
klin needs, so the rule they pin is now written down (4.4, 16.5). Three
behaviours changed with it. A finding prefers an entry it did not rise
against, so a stale accepted entry never fails a site `before` holds (4.4,
7.3). An accepted entry wins a full tie against a `before` entry at the same
site, so a merge does not turn it into a strict failure by chance of line
position (4.8). An accepted entry must give a number for every value the
gate ratchets, so no value grows unjudged behind it (4.8). ADR 0008 and 0009
carry notes.

## 1. Problem Statement

A coding agent optimizes for a green result at the end of its turn. The
cheapest routes to green are the ones a reviewer dislikes most: a silenced
check, a skipped test, a deleted test, a stub left behind, a copy instead of a
reuse, a refactor nobody asked for, a dependency that does not exist. A
language linter catches none of these, because each is legal code.

klin closes those routes with deterministic checks that run at the end of every
agent turn and, optionally, in CI. It solves four problems:

- It measures two trees and refuses only what got worse, so a project with
  existing debt adopts it on day one and stays green.
- It needs no file to start. Every fact it needs it derives from the tree and
  prints. A person pins a fact in `klin.json` only to override it.
- It hands each failure to the agent as its next task, with a remedy that adds
  or fixes code rather than deleting work.
- It runs offline, without a network, a database or a service, in the time
  budget of a hook.

Boundaries:

- klin is not a linter. Language linters and type checkers own style and
  types. klin ratchets what they report, and adds the checks they cannot make.
- klin is not a test runner. It reads results a suite already wrote.
- A local hook is feedback. Only a CI run on a checkout the agent never touched
  is a control. Section 15 names the two conformance levels.

## 2. Goals and Non-Goals

### 2.1 Goals

- Zero configuration works. `klin.json` is optional, and every value in it is
  an override of a value klin would otherwise derive and print.
- klin writes nothing into the working tree. Its own state lives in the git
  directory or in a cache directory the person names.
- One install step per host. For Claude Code that step is the plugin. For
  every other host it is the binary plus one command that writes the hooks.
- A run compares two trees and fails on new or worsened debt only.
- A ceiling derived from the tree makes day one green and close to what the
  project already does. A dated schedule a person pins once makes it tighter
  from then on.
- A ceiling change never fails a site the base holds, so tightening is safe by
  construction.
- Every check is deterministic: same two trees, same configuration, same
  binary, same output, on any machine.
- Every check runs offline, in-process where a parser is needed, and finishes
  inside a hook's time budget.
- The hook protocol works for Claude Code first and for any host that offers a
  session-start event, a pre-tool event, a prompt event and an end-of-turn
  event.
- CI is optional. When present, it is authoritative.

### 2.2 Non-Goals

- Matching another tool's numbers. A ratchet needs self-consistency only
  (ADR 0001).
- Judging style, formatting or naming. Linters own those.
- Executing a project's test suite, or running anything five times. The
  section 13 performance fixture is the explicit measurement exception.
- An MCP server, a web UI, a dashboard or a service (ADR 0008).
- Preventing a determined person or agent from working around a local hook.
- Forcing an agent to pay down debt it did not add. Section 7.3.

## 3. System Overview

Nine components, in one binary.

1. **Config** loads `klin.json` when it exists, resolves paths against its
   directory, and rejects a key klin does not know.
2. **Survey** derives every value the config does not pin: source roots,
   languages, documents, manifests, test roots, and ceilings. Every number
   comes from the derivation commit of 6.6, over the paths that commit's own
   survey holds, so an agent cannot move a ceiling by editing the `after`
   tree. Roots, languages, documents and manifests are the union of that
   survey and a walk over the `after` tree, so a new directory or a first
   file in a new language is measured on the turn that adds it. Discovering
   more never loosens a gate, because a site under a path the derivation
   commit's survey did not hold is `new` (7.1).
3. **Window Chooser** picks the two trees a run compares and says which.
   Section 6.
4. **Checks** measure one tree each and return Findings with a Site identity
   and values. A check knows nothing about the other tree. `inventory` is the
   one exception: its measure of `after` reads the `before` sites, because
   the value it ratchets is whether a site still exists (16.4).
5. **Ratchet Engine** matches Findings between the two trees plus the accepted
   list, and sorts each into new, worsened or held (ADR 0009).
6. **Gate Runner** runs every applicable gate cheapest first, prints a status
   row per gate, and returns one exit code.
7. **Host Adapter** reads a host's hook event and writes the decision in the
   host's shape. Section 9.
8. **Guard** refuses or questions an agent's tool call that would change the
   configuration, the hooks or the code owners.
9. **Reporter** prints text for a person, JSON for a harness, and SARIF for a
   code scanning host.

Three layers hold these. The measurement layer is checks and the ratchet
engine. The policy layer is config, survey and the window chooser. The
integration layer is the runner, the host adapter, the guard and the reporter.
The measurement layer MUST NOT read a host event, and the integration layer
MUST NOT read source code.

## 4. Core Domain Model

### 4.1 Tree

A tree is a set of files at one point. A run holds exactly two: `before` and
`after`. `after` is the working tree unless a command names a commit. `before`
is what the Window chose. A tree MUST be readable without a network.

### 4.2 Window

A window names the pair of trees a run compares and where each came from.
Three kinds exist.

| Kind | `before` | `after` | Who uses it |
|---|---|---|---|
| `turn` | the tree as it stood when the current turn's window opened | working tree | the Stop hook, `klin radius` |
| `branch` | the merge-base with the default branch, or the pull request base | working tree, or HEAD in CI | `klin gate` by hand, CI on a pull request |
| `push` | the push event's previous commit | HEAD | CI on a push |

Every run MUST print the window it used, as one line, before any gate output.
Every JSON report MUST carry it.

### 4.3 Derived

A value klin computed from the tree because the config did not pin it. Every
derived value MUST be printed with the word `derived` and the rule that
produced it, on the run that uses it. Two kinds exist.

A derived number, such as a ceiling, a radius percentile or a build command,
MUST be a pure function of one commit, the derivation commit of 6.6, and the
binary version, so it is cached by commit id. It is computed over the paths
that commit's own survey holds, never over a path found only in `after`.

A derived path set, such as `roots`, `languages`, the documents of
`doc_size` or the manifests of `build`, is the union of the derivation
commit's survey and a discovery walk over the `after` tree. The walk reads
names and extensions only, prunes the default skip set and every path
`.gitignore` excludes, and is not cached. Between the two trees a path set
MAY only grow. For a path the derivation commit's survey did not hold, the
check's own rule applies (5.4): complexity holds it to the floor, and
`doc_size` does not judge it and prints a NOTE. A site under such a path
that is judged matches nothing in `before` and is `new` (7.1).

### 4.4 Site

The identity a Finding is keyed by. A site is a file plus the text of its
declaration line, with the line number as a tie-breaker only (ADR 0008). A
check MAY define a different identity when a declaration line does not exist,
and MUST document it. Moving code within a file MUST NOT create a new site.
Renaming a file MUST NOT create new sites (ADR 0009). A finding MAY name a
site that `after` no longer holds, when the check ratchets existence (8.2).
Its `file` and `text` are then the `before` site's.

When one site has more than one finding or more than one entry, the findings
and the entries at that site are paired one to one, greedily, in this rank
order (16.5):

1. No ratcheted value rose against the entry.
2. The most ratcheted values exactly equal.
3. The smallest line distance. The matcher does not read a line from an
   accepted entry, so its distance is 0.
4. The finding's line. Findings arrive sorted by file then line (12).
5. Entry order: the accepted list in config order, then the `before` sites
   by line.

A finding matches at most one entry, and an entry at most one finding. A
finding is `worsened` only when no untaken entry at its site holds it. So a
stale accepted entry never fails a site that `before` holds (7.3), and a
value a person raised in the accepted list holds the site. The order also
lets a twin that moved keep its entry over a nearer twin whose values
changed, so moving code does not read as new debt, and it makes a twin
inserted between two twins the new one.

A function moved between files with its body unchanged SHOULD match its old
site. The RECOMMENDED second pass matches unmatched findings to unmatched
entries by a whitespace-normalized body hash, across files. This is additive
to ADR 0008 and does not change the primary key, so a move within a file and
a rename of a file still match on the primary key alone.

The body is everything below the first line, with every run of whitespace
collapsed. A declaration that fits on one line has nothing below it, so the
whole line is its key. A declaration that wraps keeps its later lines,
because the primary key is the declaration line and not the whole signature
(ADR 0008). Dropping the first line drops the name, so a rename whose body
did not change matches too, which is what `inventory` reads to tell a
renamed test from a deleted one (8.2). An edit to the body changes the hash,
so a move that also edits is `new`. This is an equality match on an
unchanged body and not a similarity match.

Only an entry whose site the `after` tree no longer holds is a move target.
From one site the second pass may take as many `before` entries as that site
lost, which is the count of its `before` entries less the count of its
findings, lowest line first. An entry an accepted one outranked is not lost,
so a copy is not a move: the original takes the primary match, and the copy
is `new`. klin drops a body hash from an accepted entry, so the accepted
list keeps the keys 4.8 names and no accepted entry is matched this way.

Pairing is one to one and ranks by 16.5 without the line distance, which
means nothing across files. A failure whose entry sits in another file names
that file after the values it was, so a function that moved and grew does
not read as a regression where it now sits.

### 4.5 Finding

One site with the values a check measured there. Fields:

- `file` (string) repository-relative path, REQUIRED
- `line` (integer) 1-based, REQUIRED where a line exists
- `text` (string) the declaration line, REQUIRED, part of the identity
- `values` (object) metric name to number or string, REQUIRED
- `outcome` (`new` | `worsened` | `held`) set by the engine, never by a check.
  A note about a site, such as an unmatched accepted entry or an unparsed
  file, is not a finding. It is a separate record under `notes` in the JSON
  (11.2), and its `outcome` names the kind of note rather than a verdict.

A value the check ratchets on is one where higher is worse. A check MUST name
those values. A value that is not ratcheted is carried for the report only.

### 4.6 Check and Gate

A check is one measurement and the judgement behind it. A gate is one
configured or derived instance of a check. A check declares:

- `name`, the command name and the `--gate` name
- `section`, the config key it reads
- `needs`, what the check needs of the base: nothing, the commit the window
  names, or that commit laid out as a tree beside the working one. Every check
  with a `derive` MUST need the tree, so that a tree with no config is green
  (5.1). A check that judges one tree against a number exists only for a
  number a person pinned. A check that needs the commit or the tree is the
  kind `--strict` reaches, because it has a comparison or an accepted list to
  judge. `sarif` needs the commit and not the tree: it reads which lines the
  window changed and runs the scanner once, over the working tree only (8.3).
- `takes_scope`, whether a changed-file list narrows it
- `gate_per_entry`, whether the section is a list of entries a person writes,
  each its own gate under its own `name`, rather than one section the whole
  check runs under. Only `sarif` sets it (8.3).
- `derive`, a function from a surveyed tree to a section, or none when the
  check cannot apply without a person, such as `sarif`
- `cost`, an ordinal that orders the run cheapest first

A check with no `derive` runs only when its section is present. Every other
check runs unless its section is `false`.

### 4.7 Ceiling

The value a measure may reach before its gate fails. A ceiling is derived, a
pinned number, or a pinned dated schedule. Section 7.3.

### 4.8 Accepted entry

Debt a person allows, keyed like a site, with every value the gate ratchets
and the amount of each they allow. Only a person writes one, in a reviewed
commit (ADR 0009). An entry that does not give a number for each of those
values is a config error, because a value it leaves out would grow unjudged
at that site. An entry that matches nothing is a NOTE, and a failure
under `--strict`.

The accepted entry takes the match when the finding holds against it, unless
a `before` entry the finding also holds against shares more values with it
(4.4). When the finding rose against the accepted entry but not against
`before`, or when `before` shares more values, the `before` entry takes the
match and the accepted entry matches nothing. That is how `--strict` tells a
person to delete the line once the code has moved off the accepted value.

### 4.9 Verdict

One run's result. Per gate: `ok`, `FAIL` or `ERR`. Overall: exit 0, 1 or 2.
Exit 1 means a gate failed. Exit 2 means klin could not measure something.

### 4.10 Note

Something klin reports and never fails on. A note has no ceiling.

## 5. Configuration Contract

### 5.1 The file is optional

`klin.json` at the repository root, when it exists. Discovery walks up from
the working directory. `--config PATH` overrides. Paths resolve against the
file's own directory. The file is under the guard and SHOULD be under
CODEOWNERS.

With no file, every check with a `derive` runs over derived sections. When
the two trees are the same, this MUST produce a green run, because nothing in
a tree is worse than itself. That is the first stop of the hook, whose first
stamp is the working tree as it stands. When the trees differ, as under `klin
gate` by hand against a merge-base, the run fails on new or worsened findings
only. A stale citation, a missing lockfile entry or a long document that the
base already holds is `held`, not `new`. A build that already fails, or a
file no grammar reads, is judged under section 14 and is not part of this
promise.

Under `--hook` the file is the marker that the repository opted in. When no
`klin.json` resolves, `klin gate --hook` MUST exit 0, print nothing and write
no state, before it surveys the tree. A `--config PATH` that names no file
reads the same way. `klin gate` without `--hook` keeps the exit 2 that names
the file and the sections that would fill it. ADR 0028.

One configuration per repository, at the root. Sections carry roots, so a
monorepo is many roots in one file. Discovery walks up only to find that file
from a subdirectory. A configuration per package is not supported.

### 5.2 Top-level keys

- `project` (string) OPTIONAL, a name for reports
- `version` (string) OPTIONAL. A run under another klin version prints a
  NOTE naming both and continues. The base commit removed the drift problem
  the pin existed for, so exit 2 is not warranted.
- `build` (string or list) OPTIONAL, ADR 0012. Derived from manifests when
  absent.
- `accepted` (list) OPTIONAL, section 4.8
- `radius` (object) OPTIONAL, ADR 0014, with `lines` and `directories` as whole
  numbers. Derived from history when absent.
- `journal` (object) OPTIONAL, with `prompt` a boolean. `false` turns off the
  prompt excerpt of 11.4; the excerpt is recorded by default. A configuration
  klin cannot read carries no excerpt either: the one case where klin cannot
  see this setting is the case where it MUST NOT record the text.
- `gates` (list) OPTIONAL, section 8.3. Each entry is a `name`, a `check`, a
  `with` and an optional `off`, and each is its own gate.
- one key per gate, named for its section, or `false` to exclude the gate

A key klin does not know MUST be an error naming the key. A section with a
`baseline` key MUST be an error saying the key is gone (ADR 0009). A section
MAY pin some keys and leave others to derivation. A pinned key MUST print as
`pinned` beside the derived ones.

### 5.3 One key vocabulary

Every section uses the same names for the same things: `roots`, `languages`,
`exclude`, `skip_dirs`, `ceilings`. `sources` in the complexity section is
renamed to `roots`. A run that meets the old name MUST say what the new name
is. The differential test that required the old vocabulary is retired.

### 5.4 Derivation rules

Each check documents its rule. The rules for the shipped checks:

- `roots`: directories under the tree root that hold source files of a known
  language, excluding the default skip set, merged up to the shallowest
  directory that holds nothing but source. The set is the union of the
  derivation commit's survey and the `after` walk (4.3). Test roots are the
  subset whose name or files match the language's test convention.
- `languages`: the languages of the files under `roots`, in the derivation
  commit and in `after`.
- `stubs`: the same `roots` and `languages` as `escapes`, less every language
  the stubs table holds no rows for, so a language only the escapes table
  names does not refuse the run. A tree with no language left gets no
  `stubs` section, and the gate needs a section a person writes.
- `doc_size`: every Markdown file at the tree root, in the derivation commit
  and in `after`. The
  ceiling is the word count at the derivation commit, rounded up to the next
  50 and never below 50, so an empty document gets 50 rather than a ceiling
  its first word breaks. A document the derivation commit lacks is not judged
  on that run. A
  NOTE names it and its word count, and it gets a ceiling when the stamp
  moves and the derivation commit holds it. Any other rule would read the
  ceiling from `after`, which 4.3 forbids.
- `complexity.ceilings`: the 95th percentile of `cc` and of `lines` over
  every function under the derivation commit's own roots at that commit,
  rounded up to the next whole number, with a floor of `cc 5` and `lines 25`
  so a small clean tree is not held to a ceiling of 1. Below 50 functions the
  floor is the ceiling. A root or a language the derivation commit lacks has
  the floor as its ceiling, and a function found only in `after` never
  enters the percentile.
- `radius`: the 90th percentile over the last 200 non-merge commits, per
  ADR 0014, or no section below 50 commits.
- `build`: one entry per manifest, per ADR 0012. Manifests are a path set.
  A manifest the derivation commit lacks gets its entry from the fixed table
  on the turn that adds it.

A derived ceiling is not monotone. A percentile falls when simple functions
arrive and rises when simple functions leave. A tree of 96 simple functions
and four complex held ones has a ceiling at the floor. Delete 46 of the
simple ones and the next derivation puts the ceiling at the complex four,
though no surviving function got worse. klin keeps no history that could
prevent this, and a run prints the ceiling it used, so a person sees the
number move. The derived ceiling is the day-one default and nothing more. A
person who wants a ceiling that cannot loosen pins one. Section 7.3 names
what tightens.

### 5.5 Pinned ceiling shape

A pinned ceiling is either a number or an object of dated steps:

```json
"ceilings": {
  "cc": 12,
  "lines": { "2026-09-08": 90, "2027-01-01": 70, "2027-07-01": 60 }
}
```

The run uses the lowest step whose date is on or before today. A schedule
with no step yet due MUST be an error. Today is the system date in UTC, so
two machines on one day agree on the step. A run MAY take
`KLIN_TODAY=YYYY-MM-DD` for tests. A run that uses a schedule MUST print the
date it used on the `derived:` line for that ceiling. This is the shape issue
#87 proposes for one gate, applied to every ceiling.

### 5.6 Exclusion

A gate is excluded by setting its section to `false`. Naming an excluded gate
on the command line is exit 2.

### 5.7 What `init` does now

`init` pins. It runs the survey and writes the derived sections into
`klin.json`, so a person can see them, edit them, and put them under review.
It MUST write only the config. It MUST NOT overwrite an existing config
without `--force`. `init --add` fills in missing sections and leaves `false`
alone. `init --force` re-pins every derivable section from today's tree, which
is how a person re-pins after the tree improved. It keeps what klin cannot
derive: the `accepted` list and any dated schedule, because a schedule is the
tightening a person pinned once (7.3). The guard denies `init` in
every form from an agent, so `--force` is a person's flag. `init` MUST NOT
edit `.gitignore`, because klin writes nothing that git could see.

`init --hooks` writes the hook entries for each host it detects, or for the
host `--host` names. `init --hooks --global` writes the host's user-level
file instead of the tree's own, so one install covers every repository and no
project file carries klin. Section 19.3. The hook file is not guarded (9.4),
but `init` in any form is a person's command, so the guard refuses this one
from an agent like it refuses `init` itself.

`init` is a convenience, not a step. A tree with no `klin.json` is fully
gated.

### 5.8 The configuration reference

`klin reference` prints the configuration reference as Markdown on stdout and
exits 0. It reads no configuration and no tree, so it runs anywhere.

Every check declares its keys beside the code that reads them, and the
reference prints one table per section. Each row states the key, what it
holds, whether it is required, whether the survey derives it or only a person
pins it, the derivation rule of 5.4 where there is one, and the default where
there is one. The reference states the top-level keys of 5.2 the same way, and
the dated ceiling shape of 5.5.

Every key the reference names is read through its declaration and written by
the survey through the same one, so a key renamed in the declaration is
renamed at both ends. A key inside one of them, such as the `run` of a `build`
entry, is stated in what the key above it holds and is not a row of its own.
The sections the reference prints, and the language names it prints beside
them, come off the same table of checks a run gates from, so a check cannot be
gated and left out of the reference.

A row of a section the survey supplies entry by entry says `derived with the
section`, because the rule holds only when the section itself is absent: an
entry a person pins must state the key.

The reference MUST also state what the key tables alone do not say:

- the language names of each check that selects by language, with the
  extensions each name selects, printed from the tables in the binary. The
  checks share language names and not file sets: for `complexity`,
  `typescript` selects `.ts`, `.mts`, `.cts` and `.tsx`, and `.js` needs
  `javascript`, while for `escapes` and `stubs` either name selects both sets.
- that `skip_dirs` adds to a shared default list, which it prints, and that
  `exclude_except` answers `exclude` globs only. It cannot bring back a file
  under a skipped directory, and only `complexity` reads it.
- what else a walk drops and the key tables do not say: the files git ignores,
  the dot directories that `complexity` and `inventory` skip and the other
  checks read, and the shape an `exclude` glob takes. A glob is matched
  against the basename and against the absolute path, so a glob written from
  the tree root matches nothing.

`docs/REFERENCE.md` holds the printed reference, and a CLI test fails when the
committed copy differs from what the binary prints, so CI fails on a reference
that drifted. A person regenerates it with `klin reference > docs/REFERENCE.md`.

## 6. Window Selection

### 6.1 The hook uses the turn window

In `--hook` mode `before` is the turn stamp. The stamp is a git tree object of
the working tree, including uncommitted and untracked files that `.gitignore`
does not exclude. `after` is the working tree. Committing inside the turn does
not move either.

This reverses ADR 0009 for the hook only. The merge-base is the right window
for a pull request and the wrong one for a turn. On the default branch with a
remote it measures unpushed commits and then nothing. With no remote it
measures uncommitted work and then nothing. On a long branch it re-judges
every file the branch touched at every stop. The turn window has none of
these, and it is the window ADR 0014 already built for radius.

### 6.2 When the stamp moves

One rule, applied on session start and on every prompt submitted alike:

> The stamp moves to the current working tree on a first session, or when
> the last stop ended green. Otherwise the stamp stays where it is.

So debt an agent left behind stays `new` until it is fixed or a person accepts
it, across turns and across sessions. The stamp persists in the state
directory between sessions, and a session start that finds a red verdict
keeps the old stamp rather than photographing the mess. A first session is a
worktree whose state directory does not exist yet. Its first stamp is the
working tree as it stands, which treats a person's uncommitted work as prior,
and that is correct.

An acceptance does not move the stamp. An accepted entry is a `before` entry
under 7.1, so the site it names is `held` on the next stop, and the stamp
moves when that stop ends green. Accepting finding A while finding B is still
open leaves B `new`. A rule that moved the stamp on acceptance would make B
inherited debt, which is the route to green this section exists to close.

A person who abandons the work has a third route: `klin turn reset` moves the
stamp to the current working tree and prints that a person moved it. The guard
denies the command by name from an agent, the way it denies `init`. Without
this command a red window that nobody acts on degrades into a report that
everyone learns to ignore.

A stamp is missing when the state directory exists and holds no `turn` file.
A `turn` file that is gone while the ref of 6.5 remains is restored from the
ref with a RED verdict, and a NOTE says so. When the file and the ref are
both gone, the stamp was deleted. The rule above writes no fresh stamp then,
because a fresh stamp would photograph whatever the deletion was meant to
hide. The next stop prints a NOTE that names the missing stamp, judges a
branch window from the base of 6.3, and writes that base as the stamp with
the verdict the gates gave. So deleting a stamp widens the window to the
whole branch. When no base of 6.3 resolves either, the stop judges from HEAD
and says so.

The stamp holds the stamped commit id, the time, the verdict of the last
stop, a prompt counter, the prompt mark of 6.2.1, and the `asked` record of
8.2. `klin gate --hook` writes the verdict and adds to `asked`. A fresh stamp
holds an empty `asked`, so the record goes whenever the stamp moves. `klin radius` applies the rule above and raises the
counter by one on every session start and prompt submitted, whether or not
the stamp moved. The counter is what makes "once per turn" in 9.3 literal,
because the stamp itself moves only after a green stop. The state directory
the stamp sits in is guarded (9.4).

#### 6.2.1 The prompt mark

The stamp waits for a green stop, and the radius report cannot. Keyed to a
frozen stamp, the report measures one window that grows with every prompt,
calls it the turn that just ended, and prints on every prompt once that
window passes its value. ADR 0024.

So klin takes a second mark. The prompt mark is a commit over the same tree
the stamp commits, with HEAD as its parent, under `refs/worktree/klin/mark`.
It moves on every session start and on every prompt submitted, whatever
verdict the last stop left. It is held in the `turn` file beside the stamp and
written in the same atomic write.

The prompt mark is the window the radius report measures, and nothing else
reads it. A stop judges the stamp, `klin turn reset` moves the stamp, and the
derivation commit of 6.6 comes from the stamp. A mark the `turn` file has lost
is read from the ref. With neither, the report prints nothing, which is what
it does with any window it cannot measure. Deleting the mark costs a report
and no block, so the mark needs no recovery beyond its ref.

### 6.3 `klin gate` by hand and in CI

Candidates, in order:

1. `GITHUB_BASE_REF`: the pull request target, as
   `refs/remotes/origin/<target>` then as a local name. Kind `branch`.
2. The push event's `before` commit from `GITHUB_EVENT_PATH`, when it is not
   the null commit. Kind `push`.
3. The merge-base of HEAD with the default branch, trying the remote HEAD
   symbolic ref, then `origin/main`, `origin/master`, `main`, `master`. Kind
   `branch`.

The first candidate that resolves wins. None resolving is exit 2 with the list
of what was tried. `--base REF` overrides the list.

### 6.4 A base equal to HEAD

Outside the hook, when `before` equals HEAD and the working tree is clean:

- A remote source passes and prints that the trees are the same.
- A local source passes only when the remote default branch holds HEAD.
  Otherwise exit 2 naming the unpushed commits.
- No remote tip is exit 2 under `--strict` and a pass otherwise.

This is ADR 0013 unchanged, and it now applies to `klin gate` and CI only.

### 6.5 Materialization

Under `--changed` the `before` version of each changed file comes from `git
show`, at its old path for a rename. Without `--changed` the `before` tree is
a detached worktree under a temporary directory, removed after the run. A root
the `before` tree lacks measures nothing there.

The turn stamp is a commit made with `git commit-tree` over a tree from a
temporary index, with HEAD at stamping time as its parent, so both paths above
work on it unchanged. The RECOMMENDED stamping sequence is `git add -A` with
`GIT_INDEX_FILE` pointing at an `index` file in the state directory, then
`git write-tree`, then `git commit-tree -p HEAD`, then `git update-ref
refs/worktree/klin/turn <commit>`. The ref keeps `git gc` from pruning the stamp,
makes it visible to `git log --all`, and is the copy a stop restores the
`turn` file from when that file is gone (6.2). The ref is never pushed. The `turn` file
in the state directory holds the time, the verdict, the prompt counter and
the `asked` record of 8.2 beside the commit id, and `intervened`, set once a
stop under the stamp spends a gate block (9.5). A fresh stamp holds neither.
It MUST be written to a temporary name and renamed into place, so a hook that
dies mid-write leaves the previous stamp, not a torn one. Two sessions in one
worktree share one window and one `turn` file. A stop MUST hold an advisory
lock on the state directory from before it measures until after it writes the
verdict, so stops in one worktree run in order and the last verdict describes
the last tree. Without the lock an old green stop that finishes after a new
red one would write green, and the next prompt would move the stamp over the
red debt. A stop that cannot take the lock within the hook budget writes no
verdict and says so.

The ref is `refs/worktree/klin/turn`, not `refs/klin/turn`. Git shares
`refs/` across the worktrees of one repository, with `refs/worktree/`,
`refs/bisect/` and `refs/rewritten/` as the exceptions, so a stamp under
`refs/klin/` in one worktree would replace the stamp of another and leave it
for `git gc` to prune.

### 6.6 The derivation commit

Derived values come from one commit, the derivation commit. Under the turn
window it is the stamp's parent, which is HEAD at the time the stamp was
taken. Under the branch and push windows it is `before` itself. It is never
the stamp, because a stamp is a new commit on every turn and a cache keyed by
it would never hit.

The survey MUST cache its result under `survey/<commit>.json` in the state
directory, because its numbers and its path sets at that commit are a pure
function of the derivation commit and the binary version. The `after` walk
of 4.3 is not cached and is unioned in at run time. The first stop after a
commit pays one whole-tree parse.
Every stop between two commits reads the cache. The cache is per worktree
like the rest of the state directory (7.4).

A commit inside a turn does not move the derivation commit. The stamp's parent
is fixed when the stamp is taken, and the stamp moves only under 6.2. So a
turn is judged against one set of derived values from start to end, whatever
the agent commits along the way. The derivation commit changes only when the
stamp moves, which is after a green stop, and a person sees the new ceiling on
the `derived:` line of the next run.

## 7. Ratchet Semantics

### 7.1 Outcomes

Three outcomes (ADR 0009):

- `new`: over the ceiling in `after`, no matching site in `before` or in the
  accepted list. FAIL.
- `worsened`: matched, and a ratcheted value rose. FAIL.
- `held`: matched, no ratcheted value rose. Pass.

Below the ceiling nothing is judged. An accepted entry is a `before` entry. A
finding matches at most one entry, and an entry at most one finding.
Identical sites match in the rank order of 4.4.

A site under a path the derivation commit's survey did not hold matches
nothing in `before`, whatever `before` holds there, so when the check judges
it at all it is `new`. Whether the check judges it follows its rule for a
path without a number (4.3, 5.4). A path that was not measured was never
held, so a directory that becomes a root, or a file that becomes a known
language, cannot bring inherited debt with it.

### 7.2 Scope

`--changed` restricts both findings and entries to the changed files, so an
untouched file's debt never reads as new or as fixed. It announces the file
count it judged. Under the turn window the changed files are the files the
window changed, and an empty set is an honest "nothing changed", not a hole,
because a commit does not empty it.

### 7.3 Tightening

The invariant that makes tightening safe:

> A ceiling change MUST NOT fail a site the base holds. A site over the
> ceiling in both trees with no ratcheted value risen is `held`, whatever the
> ceiling is.

Because of it, lowering a ceiling affects new code only. What tightens, in
order of how much work it does:

1. A person pins a dated schedule once (5.5). This is the mechanism. `init`
   MAY offer to write one from the derived ceiling to a target over a period,
   under a flag such as `--tighten 18m`, and MUST NOT write one unasked.
2. A person pins a lower number.

The derived ceiling tightens nothing. It can rise when simple code leaves the
tree (5.4). A pinned number or schedule is the only ceiling that cannot
loosen, and `init` writes one from today's derived value in one command.

Forced paydown, where a touched file must leave with less debt, is NOT
RECOMMENDED. Its only remedy is a refactor nobody asked for, which the radius
report exists to discourage.

### 7.4 What the tool writes, and where

Nothing into the working tree. `init` writes `klin.json` and hook files, and
only when a person runs it.

klin's own state is four things: the turn stamp with the prompt mark of
6.2.1, the build stamp, the survey cache, and the journal of 9.6. All are per
working tree. The cache is safe to delete. All four are guarded, because the
guard guards the directory they share (9.4). Deleting the turn stamp buys
nothing, because a stop without one judges the whole branch (6.2). They live
in the state directory:

- By default, `klin/` under the directory `git rev-parse --git-dir` returns.
  Git never tracks it, never lists it as untracked, `git clean` never removes
  it, each worktree has its own, and it goes when the repository goes.
- When `KLIN_STATE_DIR` is set, under that directory, in a subdirectory named
  by a hash of the git common directory and the worktree path. Two clones or
  two worktrees MUST never share a stamp. `~/.cache/klin` is the intended
  value.

An implementation MUST print the state directory under `klin gate --list`.
`klin cache clean` MUST remove the survey cache for the current tree, and with
`--all` the survey cache under every entry of `KLIN_STATE_DIR` whose repository
no longer exists. It MUST NOT remove a stamp, and it MUST leave the journal
alone: the journal is history, not a cache. A repository path that does not
resolve on the machine running the command, such as a worktree a container
does not mount, is no proof that the tree is gone, and a stamp is the one piece
of state whose loss lets a block go unspent.

ADR 0004's argument for the location holds in both: an agent empties `target/`
as a matter of routine and does not empty `.git/` or a home cache. ADR 0015's
reason for `.klin/` was one `.gitignore` line, and neither location needs one.

Anyone already running klin has `.klin-build-blocked` or `.klin/` in
`.gitignore`. Those lines become inert. `init` MAY say so when it sees one.

## 8. Check Catalogue

### 8.1 Criteria

A check earns a place when all four hold:

1. **Deterministic and offline.** Same inputs, same output, no network, no
   clock except a pinned dated ceiling.
2. **Additive remedy.** The fix adds or improves code. A check whose only
   remedy is undoing work is a report, not a gate (ADR 0014).
3. **Not a linter's job.** Either it needs git, a second tree, a second
   artifact such as a document or a lockfile, or knowledge across files. Or
   it ratchets a linter's own output.
4. **Names an agent failure.** Something an agent does when it wants green
   cheaply.

### 8.2 Tier 1: build these

Every check here except `sarif` compares two trees. "New against `before`"
means a site present in `after` and absent in `before` fails, and a site in
both is held. `inventory` runs the other way: a site in `before` and gone
from `after` fails, as a rise of its `missing` value from 0 to 1.

| Check | Agent failure it names | Identity | Judgement | Derivable | Status |
|---|---|---|---|---|---|
| `escapes` | silenced check, swallowed error, skipped test | file + line text | `count` rises | yes | shipped |
| `complexity` | tangled function written in a hurry | file + declaration | `cc`, `lines` rise | yes | shipped |
| `doc-size` | instruction file that grows every turn | document | words over a ceiling derived from the derivation commit | yes | shipped |
| `doc-citations` | document that cites a file that moved | document + path | new against `before` | yes | shipped, needs the base comparison |
| `radius` | unprompted wide change | turn | report only | yes | #91 |
| `stubs` | placeholder left behind | file + line text | `count` rises | yes | **new** |
| `inventory` over tests | deleted test file, deleted test function | test file path, or test function site | `missing` rises | yes | shipped |
| `lockfile` | dependency added without a lockfile entry, pin removed | manifest + name | `unlocked`, `unpinned` rise | yes | shipped, Rust, npm and Go |
| `sarif`, `after` only | anything a linter reports, on a line the window changed | file + rule + message | new on a changed line | no | shipped, section 8.3 |

`inventory` has two identities. A test file is keyed by path. A test
function is keyed like a complexity site, file plus declaration line, and
only functions the language's test convention marks count, such as a
`#[test]` item, a `test_` function or an `it(` call. That table is fixed in
the binary and 8.2.1 states it. `inventory` ratchets one value, `missing`,
which is 0 for every test site in `before` and 1 for a site `after` no
longer holds, so a vanished site is `worsened` under the one judge of 16.4.
Both identities reach that judge together and count in one unit, `test
site(s)`. The second pass of 4.4 matches a renamed test by body hash before
it counts as missing, so a rename with the body unchanged is `held`.

Removing a test is ordinary work, and deleting a failing one is the
cheapest route to green in section 1. klin cannot tell which it was, so it
asks once and does not judge the answer (ADR 0031):

- Under `--hook`, a vanished site is `worsened` and blocks the stop, and the
  remedy asks the agent to restore the test and fix the code if the test
  failed, or to say why the removal is intended and stop again. A stop that
  blocks records the site id (11.2) of every finding it reported in the
  `turn` file, beside the stamp (6.5). A vanished site that record holds is
  let through: its `before` entry carries `missing: 1`, so the one judge
  holds it, and the run lists it in a NOTE. The report names every finding
  it records, so nothing reaches that record unasked. The record goes when
  the stamp moves, and the guard refuses an agent's write to the file that
  holds it (9.4). The rule keys on `--hook` and not on the window kind,
  because a stop whose stamp was deleted judges a branch window (6.2), and a
  stamp that went takes the record with it, so such a stop asks again.
- Everywhere else, every vanished site is let through the same way and is a
  NOTE, `--strict` included.

An accepted entry that names a vanished site still takes the match, by
entry order (16.5), so it neither fails nor turns into an entry that matched
nothing. Nothing klin prints asks for one.

A deletion that removed the subject too is a NOTE: for a file, the subject
file went in the same window, and for a function, the file that held it
went. The subject of a test file is
the file in `before` whose path equals the test's path with the test
affixes stripped: a `test_` or `spec_` prefix, a `_test`, `_spec`, `.test`
or `.spec` suffix before the extension, a `Test` or `Tests` suffix on the
basename as in `FooTest.java` or `FooTests.swift`, and a `tests/`, `test/`,
`spec/` or `__tests__/` directory segment. The table is fixed in the binary
and printed with the NOTE.


`lockfile` proves one thing: every dependency the manifest names has an entry
in the lockfile beside it, and no pin the base held is gone. It cannot prove
that a package exists in a registry, because it runs offline. A dependency
that does not exist fails the project's own install, which the `build` step
runs. Workspace members, path dependencies and optional dependencies are
implementation-defined and MUST be documented per manifest format, which
8.2.1 does for the three formats that ship.

The first version covers the three formats that need no parser klin does not
already carry: `Cargo.toml` against `Cargo.lock`, `package.json` against
`package-lock.json`, and `go.mod` against `go.sum`. `pnpm-lock.yaml` and
`yarn.lock` need a YAML reader, and `poetry.lock` and `uv.lock` need a TOML
reader, so each is a follow-up and each is a NOTE until then. A manifest
whose lockfile format klin cannot read is one NOTE per run and no finding, so
such a manifest never reads as a pass.

Two more checks belong to this tier by the criteria and are not in the core
list of section 18, because each takes weeks and carries an unsolved problem:

| Check | Agent failure it names | Identity | Judgement | Status |
|---|---|---|---|---|
| `duplication` | copy instead of reuse | block in changed lines, and tree share | `share` rises | #48 |
| `sarif` with `compare` | a linter's findings compared across two trees | file + rule + message | `count` rises | #47, section 8.3 |

`stubs` is new. Its patterns per language are a fixed table the way escapes
are, and every pattern in the first table is an executable body that does
nothing: `todo!()`, `unimplemented!()`, `pass` as the sole body of a
function, `raise NotImplementedError`, `throw new Error("not implemented")`,
an empty body on a function the language's test convention marks, and an
elision comment such as `// ...` or `# rest of the code` as a sole body. The
sole-body patterns match function bodies only, because `pass` as the body of
a class or an exception is ordinary Python. The comment markers `TODO`,
`FIXME`, `XXX` and `HACK` are in the table too, as #106 decided. A project
that tracks work in such comments sees a new one fail once, and a person
accepts it or the agent moves the note to the tracker. An abstract
declaration whose body is meant to be empty, such as a trait method or a
protocol, MUST NOT match. Identity is file plus line text, ratcheted on
`count`, exactly like escapes. It SHOULD share the escapes engine and differ
only in the table. #106 shipped the line patterns and #114 the body shapes,
which the function walk reads.

The escapes table gains three rows for test-disabling constructs it lacks:
`fit(`, `fdescribe(` and `pytest.mark.xfail`. `skipif` is not a row, because a conditional skip states which platforms a test supports. The
other focus and skip markers, `.only`, `.skip`, `xit`, `#[ignore]`,
`@Disabled`, `t.Skip` and `XCTSkip`, are already there.

#### 8.2.1 Measurement rules of the shipped checks

The table above names what each shipped check measures. This section states
how, so a second implementation reproduces klin's own numbers and so a rule
that looks wrong is disputed as a rule, not rediscovered in source. Every
rule here is pinned by a CLI test under `tests/`, named beside it. Section 5.8
owns the configuration keys that choose roots, languages and exclusions, and
links here.

**`doc-size` counts words.** A word is a maximal run of characters that are
not Unicode whitespace, as `White_Space` defines it. A no-break space and an
em space split words the way an ASCII space does. Nothing is stripped: a
heading marker, a fence, a backticked span, an emphasis marker and every
token inside a code block are words. A byte sequence that is not valid UTF-8
is decoded lossily, and each replacement character is part of a word, never
an error. The count compares to the ceiling with `words > ceiling` failing,
so a document exactly at its ceiling passes, and lands inside the two
percent margin that adds a WARN line. Pinned by
`a_word_is_a_run_of_non_whitespace_so_markup_counts_and_unicode_spaces_split`
and `a_byte_that_is_not_utf8_is_read_as_one_word_not_an_error` in
`tests/doc_size.rs`. Known limit: the count rewards terse markup and
punishes fenced examples equally. That is a design choice for a separate
ticket, not a defect of the rule.

**`escapes` and `stubs` aggregate matches into sites.** A site is one file
plus the text of one line with leading and trailing whitespace trimmed. Every
match of every pattern in the language's table, on every line whose trimmed
text is equal, lands on that one site. Its `count` is the number of those
matches. Its label and remedy are the ones of the first pattern, in table
order, that matched a line with that text, and its line is the first line
that pattern matched. The language tables come first, in the order the
config names them, and the project's own `patterns` after them. A line that carries two kinds is one site labelled by
the earlier row, and a second copy of that line, indented differently, adds
its matches to the same site rather than opening another. `escapes` reads
the text as written, so a pattern inside a string literal is a match, and it
leaves an inline Rust test module out unless `skip_rust_tests` is `false`.
`stubs` throws away a match that lies wholly inside a quoted span on one
line, judges a test module like any other code, and refuses the key. Pinned by
`repeated_lines_of_two_kinds_fail_as_one_site_labelled_by_the_first_pattern_with_every_match_counted`,
`a_line_carrying_two_escape_kinds_counts_both_under_the_first` and
`the_same_line_twice_in_one_file_is_one_site_whose_count_ratchets` in
`tests/escapes.rs`. Known limit: the label hides the second kind on a mixed
line. A finding that says `unwrap x4` may hold two `expect` calls.

**`stubs` judges three body shapes.** The function walk of `complexity`
reads them, over the grammars of the languages the section names, and
`stubs` records each one at the declaration line of the function that holds
it, as one more match on that site. A set the project's own `patterns` make
is not a language and carries no shapes. A shape is read off a body that
holds a run of statements, so a concise arrow body such as `() => value` is
one expression and does the work of one. A function whose body holds one
`pass` statement is a `pass body`. A function whose body holds no statement,
and one comment that starts with `...` or with `rest of the`
case-insensitively once the comment markers are off it, is an `elided body`.
A function whose body holds no statement, and whose declaration the
language's test convention marks, is an `empty test`. A comment is not a
statement in any of the three.

A body meant to be empty is not a shape. A declaration that carries no body,
such as a trait method without a default or an interface method, has none. A
Python declaration under a decorator that names `abstractmethod`,
`abstractproperty` or `overload` has none, and neither does one in a class
whose bases name `Protocol` or `ABC`. Both are read as whole names, the last
segment of a dotted one, so `StoreABC` is not `ABC` and a route argument
that spells `overload` exempts nothing. `pass` in a class body, an exception
class included, is not a function body and never matches. A function that
starts on the declaration line of a function that holds it is not judged
either, because the line it would be reported at is not its own: a callback
written inside the call that declares a test carries the test's line and not
its shape. Pinned by
`a_pass_body_fails_and_the_same_declaration_with_a_body_stays_green`,
`an_elided_body_fails_and_a_comment_that_elides_nothing_stays_green`,
`an_empty_test_body_fails_and_a_test_rewritten_with_the_same_declaration_stays_green`,
`pass_on_an_exception_class_and_on_an_abstract_declaration_is_not_a_stub`,
`a_callback_on_the_line_of_a_test_declaration_is_not_an_empty_test` and
`a_decorator_or_a_base_whose_text_only_spells_a_marker_does_not_hide_a_pass_body`
in `tests/stubs.rs`. Known limit: another shape that stands in for work,
such as `return null` or `{}` on a function no test convention marks, is not
judged, and adding one is a spec change with its own legitimate-use fixture.
Second known limit: a text the grammar rejects keeps its line patterns and
loses its shapes, and the run says nothing about the loss, so a file that
does not parse can only under-report.

**`doc-citations` reads backticked paths, not Markdown links.** On each line,
backticks pair from the left, and an unpaired trailing backtick opens
nothing. A span is a citation when, after trimming and dropping everything
from the first colon on, it holds no space and no `*`, it ends with one of the
configured extensions, the default list being the source, document and
manifest extensions the module names, and it holds a `/` or a `.`. So `` `src/a.rs:12` `` cites
`src/a.rs`, and `` `*.rs` ``, `` `a b.rs` `` and `[a](src/a.rs)` cite
nothing. Any citation resolves when a root holds a file at that path. A path with
a `/` resolves nowhere else. A bare filename no root holds directly
resolves when exactly one file under the roots has that basename, is
ambiguous when several do, and resolves nowhere when none does. Identity is
the document plus the cited path after trimming and the colon strip, so the same stale string on two lines is one site with `count` 2, and the
same string moved to another line is held. Pinned by
`a_span_that_is_not_a_path_is_not_read`,
`a_wildcard_a_link_and_a_line_suffix_are_read_as_the_syntax_says`,
`a_bare_filename_resolves_when_exactly_one_file_under_the_roots_has_that_name`,
`two_candidates_is_ambiguity_reported_with_both`,
`a_stale_citation_moved_to_another_line_is_held` and
`the_same_stale_string_cited_once_more_is_worsened_with_the_count` in
`tests/doc_citations.rs`. Known limit: a citation split across two lines, a
path in a Markdown link, and a path with a space are never read.

**`complexity` measures each function on its own.** The unit is a function
node of the language's grammar, and an accessor or initializer body counts
as a function of its own. Cyclomatic complexity starts at one and adds one
for each decision node and each boolean operator in the language's tables,
walked through the function's body. A nested function is not walked: it is
excluded from the count of the function around it and measured as a site of
its own, at its own declaration line. The `default` arm of a Java or Swift
`switch`, the `else` arm of a Kotlin `when`, and a single unguarded
catch-all arm of a `match` or `case` add nothing. `lines` is the
count of source lines from the first line of the declaration to the last
line of its body, both inclusive, so a one-line function is 1. The
hand-checked numbers per language are the contract, in
`*_functions_carry_their_hand_checked_numbers` for Rust, Python, TypeScript,
Go, Java, Ruby, Swift and Kotlin, with
`a_nested_function_is_measured_on_its_own_not_folded_into_the_one_around_it`,
`a_fall_through_arm_is_not_a_decision`,
`a_guarded_catch_all_arm_is_still_a_decision` and
`an_accessor_or_initializer_body_is_measured_like_any_other_function` in
`tests/complexity.rs`. Known limit: which grammar nodes are decisions is per
language and fixed in the binary. Two languages that express one construct
differently may count it differently, and the fixtures are the record of
which choice was made.

**`inventory` ratchets the existence of two things.** A test file is the
repository path of a file the base commit's tree listing holds under an
entry, and its `missing` is 1 when the working tree holds no file there. A
test function is a site of ADR 0008 inside a file an entry holds, in both
trees, found by the walk `complexity` does and kept only where the
language's test convention marks it: a `fn`, `def` or `func` declaration
whose name starts with `test_`, a `func Test` declaration, an `it(` or a
`test(` call at the start of the declaration line, and a `#[test]`
attribute or an `@Test` annotation on the declaration line or on the run of
marker lines directly above it. A marker an identifier runs into matches
nothing, so `myfunc Test` is not a declaration, and a call marker counts at
the start of the line only, so `def helper(test_arg): return it(test_arg)`
is not a test. Where the grammar holds the annotation inside the function's
own node, as it does for Java, the declaration line is the first line of
that node that carries more than an attribute or an annotation, so the site
names the method and the body hash of 4.4 leaves the name out. Its
`missing` is 1 when no function in the working tree takes it, by site first
and then by body hash, one to one on each pass. An entry's `path` may name
a directory or a single file, and its `pattern` limits both identities the
same way. A file the working tree's grammar refuses holds no function site,
so the functions in it are not judged and the file is the unparsed refusal
of ADR 0003. Pinned by
`deleting_a_test_function_from_a_file_that_stays_blocks_the_stop_and_asks_why`,
`a_deleted_test_function_is_a_note_that_fails_nothing_outside_the_hook`,
`the_stop_after_the_question_passes_and_leaves_a_green_verdict`,
`a_prompt_between_two_stops_does_not_ask_about_the_same_test_again`,
`a_stop_whose_stamp_was_deleted_still_asks_about_a_deleted_test`,
`a_test_function_renamed_and_moved_with_its_body_unchanged_is_held`,
`a_function_whose_name_only_holds_a_marker_is_not_a_test_site`,
`a_test_name_with_no_attribute_above_it_is_a_test_site`,
`an_entry_that_names_one_file_judges_the_functions_in_it` and
`a_test_file_no_grammar_reads_is_named_and_exits_two` in
`tests/inventory.rs`. Known limit: the convention table is fixed in the
binary, so a project whose tests carry another mark has no function
identity, and only its test files are ratcheted.

**`lockfile` reads the manifest and the lockfile beside it.** A site is the
manifest's repository path plus the dependency name, and it carries two
values, both higher is worse: `unlocked` is 1 when the lockfile holds no entry
for the name, and `unpinned` is 1 when the manifest's specifier is a range or
absent. Exact means a Cargo requirement that starts with `=`, an npm
specifier that starts with a digit and holds no operator and no wildcard
segment, and every Go `require`, which states one version. A path, git or
workspace dependency has no registry behind it and no version to pin, so it
carries 0 for both values and can never fail. The manifest tables read are
Cargo's `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and
`[workspace.dependencies]`, including the `[dependencies.name]` sub-table
form, the dotted-key form, an inline table on one line or several, and any
table whose last path segment is one of those names, so a target-specific
table is read too. A statement only ever takes a value away, so two tables
that name one dependency read the same in either order, and a `features` line
undoes no version another line stated. A Cargo `package` field gives the name
the lockfile is searched for, so a renamed dependency is found. A comment is
cut off before the line is read, so a brace inside one opens no inline table.
The npm tables are `dependencies`, `devDependencies`,
`optionalDependencies` and `peerDependencies`. Go reads both forms of the
`require` directive, with a module a `replace` directive sends to a local
path treated as having no registry. The lockfile is the nearest one at or
above the manifest's own directory, which is how a workspace member finds the
one lockfile its members share. `Cargo.lock` gives the `name` of each
`[[package]]` block, `package-lock.json` gives the keys of `packages` with
everything up to the last `node_modules/` stripped and the keys of the nested
`dependencies` tree that version 1 writes, and `go.sum` gives the first field
of each line. A dependency the base manifest did not name is a site only when
it is `unlocked`, so a new dependency with a range and a lockfile entry is
not a finding, while a pin the base held and a lockfile entry the base held
are both `worsened` when they go. A manifest with no lockfile in either tree
is a NOTE and no finding, and a lockfile only the base held makes every
dependency of that manifest `unlocked`, so deleting a lockfile fails. A
supported lockfile klin cannot parse is a tool error naming the file. A
manifest the survey derived that klin cannot parse now, and that did not
parse at the base or that the base did not hold, is a NOTE naming the
manifest in every run, hook or not (8.6). It judges none of that manifest's
dependencies, and every other manifest is still judged, so a fixture that is
invalid on purpose does not turn the gate red. A derived manifest that parsed
at the base and does not parse now is a tool error naming the file, because
the work broke it and the agent can fix it. A derived manifest that did not
parse at the base and parses now is judged against a base that named no
dependency. A `manifests` list a person pinned, in `klin.json` or a `gates`
entry, asserts that every path in it parses, so there a manifest klin cannot
parse in either tree is a tool error naming the file. Only the npm reader can
reject a manifest, because the Cargo and Go readers are line scans. The check
judges every manifest under `--changed` as well, because a lockfile change
judges a manifest whose own text did not change and the whole set is a
handful of files. Pinned by
`a_new_rust_dependency_with_no_lockfile_entry_fails_as_new`,
`a_rust_lockfile_entry_that_went_fails_as_worsened`,
`a_rust_pin_that_became_a_range_fails_as_worsened`,
`a_new_dependency_with_a_range_and_a_lockfile_entry_does_not_fail`,
`a_path_a_git_and_a_workspace_dependency_are_not_judged_for_unlocked`,
`a_deleted_lockfile_fails_every_dependency_of_its_manifest`,
`a_workspace_lockfile_above_the_member_manifest_is_found`,
`a_sub_table_and_a_target_table_are_read_like_any_dependency_table`,
`a_dotted_key_states_one_field_and_undoes_nothing_another_line_stated`,
`an_inline_table_written_over_several_lines_is_read_as_one_dependency`,
`a_brace_inside_a_comment_hides_no_dependency_below_it`,
`a_renamed_dependency_is_locked_by_the_package_the_lockfile_records`,
`both_npm_lockfile_versions_hold_a_dependency_in_the_base_state`,
`an_unreadable_lockfile_format_is_a_note_and_judges_no_manifest`,
`a_malformed_lockfile_is_a_tool_error_naming_the_file`,
`a_derived_manifest_klin_cannot_parse_is_a_note_and_every_other_manifest_is_judged`,
`a_derived_manifest_that_parsed_at_the_base_and_does_not_parse_now_is_a_tool_error`,
`a_derived_manifest_that_did_not_parse_at_the_base_is_judged_once_it_parses`,
`a_pinned_manifest_klin_cannot_parse_is_a_tool_error_naming_the_file` and
`under_changed_a_changed_lockfile_with_an_unchanged_manifest_is_still_judged`
in `tests/lockfile.rs`. Known limit: the Cargo and Go readers are line
scans, so a manifest that states a dependency in a shape the scan does not
know contributes no site rather than a wrong one.

None of these rules asks another implementation to agree with klin. They
state what klin's own tests hold, per ADR 0025, so a change to one is a
change to the spec and to a test in the same commit.

### 8.3 The linter seam

The seam ships in two steps. The first needs no second tree and fits the hook
budget. The second is deferred until a user asks for the cases it catches.

**Step one, the `after` tree only.** A `sarif` entry names a command that
writes a SARIF report from the working tree, or a report that is already
there:

```json
"sarif": [
  { "name": "eslint", "run": "npx eslint -f sarif -o out/eslint.sarif .", "report": "out/eslint.sarif" },
  { "name": "semgrep", "report": "out/semgrep.sarif", "differential": true }
]
```

With `run`, klin deletes `report`, executes the command in the working tree
the way it runs `build`, then reads `report`. A report that is missing after
`run` is ERR. The report path SHOULD be under `.gitignore`, so the stamp of
6.5 does not carry it. The report is fresh by construction, because the only file at
that path is one the tool wrote over the tree klin is about to judge. The
command's exit status is not judged, because a linter exits non-zero when it
finds something. In the hook this is the RECOMMENDED form. Without `run`,
klin reads the report as it finds it, and that form belongs in CI, where the
same job wrote the report one step earlier. There, a report older than any
file the window changed is ERR, because a report that predates the change
cannot describe it. Executing the tool in the `after` tree has no dependency
problem: the working tree has its dependencies installed, or the build would
fail first.

Each entry of the section is its own gate, named by its own `name`, because
nothing in a tree says which scanner an entry runs. The section MUST be a list,
and an entry with no `name` is a config error naming the key. A `sarif` gate
reads the base commit and never the base tree, so it runs one scanner over the
working tree and no `before` worktree is laid out for it.

Each result is keyed by file, rule id and message. A result fails when its
line falls inside a hunk the window changed. A result in a file the tree does
not track is one whole changed range, so every result in it is judged.

**Where a result sits.** A result's `physicalLocation.artifactLocation.uri`
reaches klin in four forms, because every scanner writes a location its own
way: a path relative to the repository, an absolute path, a `file://` URI, and
a path under an entry of `originalUriBaseIds`, which may name a further entry
in turn. klin percent-decodes each uri, drops a `file://` scheme, resolves a
base id through at most four entries, strips the tree's own directory off an
absolute path, following the real directories both name so that a tree reached
by a symlink still places, and resolves the `.` and `..` segments of what is
left. A location that lands outside the tree, in any of the four forms, and a
result with no physical location at all, is a NOTE naming the uri and is not
judged, because a path klin cannot resolve says nothing about which lines
changed. A location with no `region.startLine` starts at line 1, which is the
line a result about a whole file is judged on.

Several results of one rule in one file are one finding with a `count`, so an
accepted entry holds that site at the count a person accepted. An accepted
entry that matches nothing is a NOTE, and exit 1 under `--strict`, as for
every other gate (10).

A report klin did not write is judged fresh by modification time: klin
compares the report file's time against the time of each file the window
changed, and a report older than one of them is ERR.

A result on a line the window did not change is held, whatever the base held
there, and the gate's `OK:` line says how many results it judged and how many
it held. With `differential: true` the tool already reports only what is new,
so every result fails and that line says instead that it judged every result
the scanner reported. Scoping to changed lines, not changed files, is what
keeps an agent that touches a file with thirty old warnings green. A reformat
that moves every line is the known weakness, and the radius report already
names such a turn.

This step runs the tool once, needs no `before` worktree, and delivers most
of what the goal names: eslint, tsc, clippy and ruff findings become klin
failures exactly where the agent wrote the line. The same `run` and `report`
contract is the one a coverage reader or a test-result reader takes later.

**Step two, `run` in both trees.** An entry marked `compare: true` is
executed in both trees and its results are matched by site and ratcheted on
`count`. This catches a rule count that rose on an unchanged line, and a
finding that moved. It is deferred for a reason the review stated: the
`before` worktree has no installed dependencies, and a symlink of today's
dependencies runs today's tool over old source, which is not a pure function
of the `before` commit and breaks section 12. A design for step two MUST
solve that before it is written. A tool that compiles the tree, such as
clippy or `tsc`, is out of scope for `compare`: a bare `before` worktree
would fetch dependencies, which section 12 forbids, and a build is outside
the budget of section 13. The first candidate is a tool that is one binary
and reads no project dependencies, such as ruff.

### 8.4 Tier 2: build when tier 1 is green

`conventions` with structural rules (#42, ADR 0006), `public-api` (#46),
`dead-symbols` and `reachability` over one reference extractor (#49, #52,
#51), `changed-coverage` and `crap` over one coverage reader with a postflight
run (#53, #54, #55, #70), `hotspots` as a report (#60), SARIF output (#65).

The reference extractor is the `syntax` module (#49, ADR 0035). It owns the
grammars, the parser and the file no grammar read, and it hands a check
declarations, imports, module declarations and references, so no check holds
another language's node kinds. It resolves a reference by name to every
declaration of that name under the roots. That errs toward "referenced", so
`dead-symbols` and `reachability` fail less, never more.

Rust and TypeScript are the first structural languages, and TSX is TypeScript
rather than a language of its own. A file in a language no structural adapter
reads is counted as not measured on the coverage line of 8.6, so a green run
over such a tree is visibly a run over nothing. Resolving an import or a Rust
`mod foo;` to a file belongs to #50, and the specifier is kept as written for
it.

A `test-hygiene` check, a count of habits across the test roots against a
dated ceiling, was considered and is not a check. A habit that rose is an
escapes `patterns` row with `roots` set to the test roots, and a schedule on
a whole-tree total fails a tree nobody changed, which is the forced paydown
7.3 refuses.

### 8.5 Tier 3: defer with a reason

`layering` (#50): defer until the reference extractor exists and a user needs
constraints beyond the compiler's module graph. `guard-suites` (#43) and
`manifests` (#44): one stack each.
`db-migration-safety` (#57): deterministic for raw SQL only. `asset-path`
(#59): the ticket expects false positives, which fails criterion 1 in
spirit. `flaky-test-runner` and an MCP server: refused in ADR 0008.

### 8.6 Per-check contract

Every check MUST:

- state its identity rule, its ratcheted values and its derivation rule in
  its module docstring
- print one remedy per failure that names what to change
- print, per failure, the site it matched in `before` or the accepted entry it
  matched, and the value on each side, with the ceiling in force beside them,
  so an agent fixes the right thing and a person can dispute a wrong match. A
  `new` finding says that nothing matched. A check that prints a held site
  says the value the base holds it at, for the same reason. Section 11.1
  fixes those line shapes.
- print one `OK:` line with what it judged on success, plus any `NOTE:`
  lines, and nothing else. What it judged includes the coverage: how many
  files it found, measured, excluded and could not read, so a green run over
  an unexpectedly small scope is visible on its one line. Section 11.1 writes
  the boundary between those four counts down once, and every check uses it.
  A check whose section is a list a person writes entry by entry, such as
  `doc_size`, says its entries on a line each and the gate's coverage on one
  line for the gate.
- name every file that is present in both trees, was measured in `before`,
  and was not measured in `after`. The union of roots in 5.4 means a check
  can only discover more, so such a file left through an exclusion, a file
  the grammar stopped reading, or a discovery rule the tree no longer meets.
  In the hook it is a NOTE. Under `--strict` it is exit 2, per section 10.
  The rule binds a check whose scope is a set of source files. A check whose
  scope is a list a person writes, a report, or the very set it ratchets,
  such as `inventory`, has nothing to lose this way that it does not already
  judge. `before` is measured under the `exclude` list the base commit's own
  configuration names for the check, and under today's where the base holds
  none: today's list applied to both trees could never show a file that an
  exclusion this run added took away. Every other rule is today's on both
  trees, so the reason a check gives is what it sees in `after`, and it does
  not reconstruct why `before` measured the file. A scoped run reports the
  loss among the files in its scope. The JSON carries the loss as a `lost`
  record under the gate's notes (11.2).
- name a file it could not measure. Outside the hook that is exit 2, with or
  without `--strict` (ADR 0021). In the hook it is a NOTE, because the agent
  has no remedy. One exception narrows ADR 0021: a `lockfile` manifest the
  survey derived, which klin cannot parse now and could not parse at the base
  or which the base did not hold, is a NOTE in every run (8.2.1). A tooling
  repository keeps such a manifest as a fixture on purpose, so it is not a
  hole the work opened, and no run could ever end green around it.
- run under `klin gate` and under its own subcommand with the same output
- carry tests through the binary only, on a throwaway tree with a base

## 9. Hook Protocol

### 9.1 Host adapter

Hosts come in two kinds, and a port starts by naming which kind the host is.

A shell-hook host runs a command with a JSON event on stdin and reads a
decision from stdout or from the exit code. Claude Code, Codex CLI, Cursor
and Hermes are this kind. Each one is one module under the host adapter that
implements the adapter's trait: the name `--host` takes, the marker
directory, the hook file, the guard's tool matcher, the key its hook file
lists plugins under if it has one, how an unlabelled event is recognised as
this host's, how the event is read, and how a decision goes back. A port of a
shell-hook host adds that module and registers it in the adapter list, and
touches nothing else. No other module names a host.

A program-hook host loads a module in its own process and has no shell hook.
OpenCode and Pi are this kind, and klin cannot be the hook. A shim outside
this crate translates the host's event into Claude Code's payload shape,
spawns `klin <command> --host claude`, and translates the answer back. The
shim is the whole port, and klin gains no variant for it.

Two facts shape both kinds. The stop answer is the report text, and the
adapter chooses the channel it goes out on: the exit code, stderr, or a JSON
field. A host's guard event carries a list of paths, because one event may
name more than one file, and the adapter maps that list onto the record
below.

One adapter, chosen from the event's shape or from `--host`, maps a host
event to one internal record:

- `tool` (string), `file_paths` (list), `command` (string) for the guard
- `blocked_before` (bool), the host's flag that the hook already blocked this
  turn
- `session` (string) when the host sends one

And maps one internal decision to the host's output. For Claude Code:
pre-tool decisions go out as `hookSpecificOutput.permissionDecision` with
`allow`, `deny` or `ask` and a reason. Stop blocks are exit 2 with the report
on stderr. A stop that ends with something for the person, such as a note
about a file no grammar read or a deleted test the run let through, writes
it as a JSON `systemMessage` on stdout under exit 0. A stop whose host event
klin cannot read writes that note to stderr and exits 1 instead, by the rule
of 16.5, because klin does not know whose shape to tell it in. Prompt and
session-start text go to stdout on exit 0.
For Codex CLI, `allow` is exit 0 and both `deny` and `ask` are exit 2 with
the reason on stderr, because Codex rejects `permissionDecision: ask` on
`PreToolUse` as unsupported. Stop blocks also use exit 2, and Codex makes the
reason a continuation prompt inside the same turn, which runs no
`UserPromptSubmit`. A stop that tells the person uses `systemMessage` too,
because Codex rejects plain text on a stop that exits 0.

In hook mode the exit code is the host's protocol, not the verdict. Exit 2
means "block this stop", whatever caused it. The verdict of section 4.9 lives
in the report and in the `turn` file. Outside hook mode the exit code is the
verdict.

The Cursor variant is a separate ticket (#67). Codex CLI sends Claude Code's
event fields plus `turn_id` on every turn-scoped event. `turn_id` alone
places an event as Codex, and it is tried before Claude Code's fields.
`permission_mode` places nothing, because both hosts send it. A session
start is not turn-scoped, so Codex's is read in Claude Code's shape, which
runs the same `radius`. Codex's `Bash` tool carries a shell command in
`tool_input.command`. An MCP tool carries its own arguments, so the guard
reads a `command` there only when the tool has one. `apply_patch` carries
one or more file paths in its patch headers. The adapter is the only module
that reads a host's JSON.

### 9.2 Events

| Event | Command | Blocks | Writes |
|---|---|---|---|
| session start | `klin radius` | never | `turn` per 6.2, its prompt counter, and the mark of 6.2.1 |
| pre-tool | `klin guard` | deny or ask | nothing |
| prompt submitted | `klin radius` | never | `turn` per 6.2, its prompt counter, and the mark of 6.2.1 |
| stop | `klin gate --hook --changed` | each stop while the build fails, up to eight per turn, and once per turn for gates | `build-blocked`, and the verdict in `turn` |

The hook lines are the same on every host and call `klin` from PATH:

```
klin guard
klin radius
klin gate --hook --changed
```

The host adapter reads which host called from the event, so no flag is
needed in the hook line. `--host NAME` overrides detection.

A stop in a tree that holds no `klin.json` prints nothing and blocks
nothing, per 5.1.

### 9.3 The block-once policy

ADR 0004 and ADR 0012 hold in policy. A build failure blocks each stop until
the tree builds. A gate failure blocks the first stop and reports on the
second. The build stamp in the state directory carries the fact between the
two processes.

ADR 0004 relies on the host's cap on consecutive blocks. That cap is not in
the current Claude Code documentation. klin MUST bound its own blocks (ADR
0022). A gate failure blocks once per turn. A deleted test is the one gate
failure that does not stay red: the stop that blocks on it records the
question beside the stamp, and the next stop lets it through as a NOTE and
ends green (8.2, ADR 0031). A build failure blocks at each
stop until the tree builds, up to eight in one turn, and then the hook
reports, says that it stopped blocking, and lets the turn end. The build
stamp holds the count and the prompt counter of 6.2 the count was taken
under. A count taken under an earlier prompt reads as zero, so every turn
has eight blocks and only the stop writes the build stamp. A build-failure
stop writes a RED verdict before it blocks, so the next prompt does not move
the turn stamp over a tree that does not build.

Two facts in ADR 0014 about where hook output goes on exit 0 need one more
check against the current documentation before #91 lands. The documentation
read for this draft says Stop-hook stdout on exit 0 reaches the model. If that
holds, the last turn of a session can carry a radius report at its stop.

### 9.4 The guard's three decisions

The guarded set is this tree's `klin.json` and this tree's own state
directory. The guard resolves both: the configuration beside the tree root
`git rev-parse --show-toplevel` names, and the state directory of 7.4. A
host's hook file, CODEOWNERS, `refs/worktree/klin`, every verification file,
another project's `klin.json` inside the tree, and another worktree's state
directory are ordinary files, and an edit to one of them is `allow`. ADR
0027, ADR 0032 and ADR 0033 record why.

The guard answers only where it can prove that a tool call writes a guarded
path. Everything else is `allow`, and a write the guard misses costs nothing,
because ADR 0009 makes CI authoritative. ADR 0033 records the reversal.

- `deny`: an edit tool whose path is the configuration or reaches inside the
  state directory, a redirect onto either, `init` in any form, and `turn
  reset`. The reason names the file and says a person changes it in a
  reviewed commit, or names the command a person runs instead: `klin turn
  reset` for the stamp, and `klin cache clean`, which stays open to an agent,
  for the cache. Nothing else denies.
- `ask`: a command that writes every argument it takes, with a guarded path
  among its arguments. The writers are `rm`, `rmdir`, `unlink`, `shred`,
  `mv`, `truncate` and `tee`, and `sed` or `perl` carrying `-i`. `cp` and
  `install` are not writers here, because each reads its first argument. The
  reason quotes the token that matched. The person decides.
- `allow`: everything else, including every command that only names a guarded
  path.

A path resolves from the directory the guard runs in, with `.` and `..` taken
out and every symbolic link its existing part carries followed. A path the
guard cannot resolve is `allow`. A token holding a shell wildcard proves
nothing about the file it stands for, so the guard matches no path against
it, and `rm *.json` in the tree root passes.

The guard matches no path in a command that holds shell it does not read: an
unbalanced quote, `$(`, a backtick, `${`, `<<`, a backslash, or a `cd`
command word. An unbalanced quote allows the whole command, because the
command then does not say where its arguments end. The other six suppress
path matching alone. `init` and `turn reset` name no path, so their `deny`
stands behind any of them, and a command substitution in command position is
a prefix in front of klin's own name.

A command splits into segments at `;`, `&`, `|` and a newline, and the split
MUST honor single and double quotes (issue #90). A redirect is an unquoted
`>`, so a `>` inside an argument is a character of that argument and not a
redirect. A writer and klin's own name are read by the basename of the
command word, and each is found behind the same prefixes: an assignment,
`env`, `npx`, `pnpm`, `bunx`, `time`, `nice` and `sudo`. So a script of the
tree's own named `rm` is read as `rm`, and `sudo rm klin.json` asks.

For Codex CLI, an `apply_patch` call is judged by every path in its `*** Add
File:`, `*** Delete File:`, `*** Update File:` or `*** Move to:` headers.
Patch body text is data. A deny for any path wins; otherwise an ask wins over
allow.

The guard MUST NOT read the configuration. It runs before the config loads.
It MAY read `KLIN_STATE_DIR` and run `git rev-parse` to resolve the tree root
and the state directory, and it reaches for git only when a path needs
proving. It MUST finish in under 50 milliseconds, because it runs on every
tool call.

### 9.5 What the hook prints

On a block, one lead line that says how many gates failed and what to do,
then each failing gate's own output, then the derived values the run used.
Never the command that accepts debt, and never `turn reset`. On a second stop,
the same report and a line saying the window stays open until a person fixes,
accepts or resets it.
The `--json` form is available for a host that reads JSON.

A stop that nothing blocks can end a turn, and on a turn with an
intervention (11.5) it tells the person what happened in one `systemMessage`
of 9.1. A green stop names the count the agent fixed: `klin: the agent took 2
shortcuts this turn and fixed both after klin asked.` A red pass-through names
what is still there: ``klin: one shortcut is still there. `klin stats --turn`
names it.`` The line and the headline of 11.5 come from the same words. A turn
with no intervention prints no such line, and a session start prints none. A
fix in a later prompt of the same turn still gets its line, because the turn
stamp records the intervention (6.5) until the stamp moves.

At most once every seven days, and only when the journal holds seven days,
one sentence about the week follows the line, with the command that lists it:
``In the last seven days, klin caught 9 shortcuts and the agent fixed 8 of
them on its own. `klin stats` lists them.`` Where the agent fixed none, the
sentence ends after the count. A command in the message stands in backticks,
and the message never names a command that accepts debt. The notes the run
left (8.2, 14), the turn-end line and the weekly line join in one message, in
that order. The stop reads the journal for this only where the turn stamp
records an intervention, or where the prompt's gate block is spent (16.3),
and its journal line records which parts it printed (11.4).

### 9.6 The journal

Every `klin gate --hook` stop MUST append one line, the record of 11.4, to
`journal.jsonl` in the state directory — the stop that blocks, the stop that
passes, and the stop that could not run its gates alike. Two stops write
nothing: a tree that holds no `klin.json` runs nothing, per 5.1, and a config
only a person can fix ends the stop before it reads anything, per 14.

Three more events append the other kinds 11.4 defines. `klin radius` on a
`UserPromptSubmit` event MUST append a `prompt` line; a `SessionStart` event
moves the mark and raises the counter of 6.2 the same way but appends no
line of its own, because it opens a window and ends no turn. The guard MUST
append a `guard` line for an `ask` or a `deny`, and MUST append none for an
`allow`, because the guard runs on every tool call under its 50 millisecond
budget (13) and an allow tells a reader nothing. `klin turn reset` MUST
append a `reset` line.

The write MUST NOT change a block or a pass: it is best-effort, a failed
append prints nothing to the agent, and a state directory klin cannot write
costs the record and nothing else, by the rule of 14. The hook never prunes
the file, and `cache clean` leaves it alone (7.4). Only the reader of 11.4
tolerates what an interrupted writer can leave: a truncated last line.

## 10. Runner and CI Contract

- `klin gate` runs every applicable gate cheapest first and prints a status
  row per gate, the full output of each failing gate, and one summary line.
- `--gate NAME` runs one gate. Naming an excluded or unknown gate is exit 2.
- `--strict` adds four failures: an accepted entry matching nothing, a
  same-tree comparison klin cannot explain (6.4), a file measured in `before`
  and not in `after` though present in both (8.6), and a survey that finds no
  source root. The last one closes the hole the retired unaccounted-gate
  failure of ADR 0010 used to close: a CI job in the wrong directory or over
  a clone with no base would otherwise apply every gate to nothing and print
  green. A config error is exit 2 in every mode but the hook (14), so it is
  not on this list.
- `--hook` with `--strict` is a usage error: the reason goes to stderr with
  exit 1, which no host reads as a block. The two flags name two callers.
- `--changed` scopes to the window's changed files.
- `--list` prints applicable gates with `derived` or `pinned` per section,
  excluded gates, and gates that need a section a person writes.
- CI SHOULD run `--strict` and MAY name gates on the command line. Naming
  them is no longer required to catch a deleted section, because a deleted
  section is a derived one.
- CI MUST fetch enough history to resolve the base. `fetch-depth: 0` is the
  RECOMMENDED setting.

## 11. Output Contract

### 11.1 Text

Stable, tested line shapes. `ok    NAME`, `FAIL  NAME`, `ERR   NAME` for
rows. `OK:` for a passing gate's one line, printed under its row by the
runner and on its own by the gate's subcommand, because 8.6 asks for the same
output from both. `FAIL:` for a failure with its remedy under it. `NOTE:` for
a note. One `window:` line first. One `derived:` line per derived value,
after the rows.

Every `OK:` line ends in the gate's coverage, as `(N file(s) found, N
measured, N excluded, N unreadable)`. The boundary is the same for every
check: `found` counts every file the check's own discovery rule reached under
its roots, before anything dropped one; `excluded` counts the ones an
exclusion dropped; `unreadable` counts the ones it reached and could not read
or parse; `measured` counts the ones it judged. A scoped run counts only the
files in its scope. A check whose scope is not a set of files counts the
thing it discovers — a document, a manifest, a test file the base holds — and
its module docstring names that thing.

Every failure line ends in what the ratchet judged it against: `— matched the
base site at FILE:LINE`, `— matched the accepted entry for FILE`, or `—
nothing matched` for a `new` finding, with `, ceiling C` after it wherever
the gate has a ceiling of its own. So an agent fixes the site the ratchet
compared, and a person can dispute a wrong match.

### 11.2 JSON

One object on stdout. Fields:

- `status`, the row of 11.1 for the run as a whole, and `summary`, the one
  line 11.1 ends with
- `window` `{kind, before, after, how}`
- `derived` list of `{section, key, value, rule}`
- `gates` list of `{name, status, findings, notes, coverage, ms, held}`,
  where `status` is the row of 11.1, `findings` and `notes` are how many that
  gate left in the two lists below, `coverage` is the
  `{found, measured, excluded, unreadable}` counts of 11.1, or null for a
  gate that could not run far enough to measure a scope, `ms` is how long the
  gate's own measure and judge took, and `held` counts the findings the run
  let through because a base site or an accepted entry carried them, which is
  one quantity and not two: a gate that also drops sites its window never
  reached counts those in its `coverage` and never in `held`. On a passing run
  it is the count the gate's `OK:` line of 11.1 prints as held at the base, and
  it is null for a gate that never got that far
- `findings` entries per 4.5 with `id`, `condition`, `fix_advice`,
  `ceiling`, and `matched`, which is the `before` site or accepted entry as
  `{file, line, text, accepted, values}`, or null for a `new` finding. The
  list carries the run's failures, so a `held` site is not in it: a held
  finding is not a failure, and the gate that prints one says so on its own
  `OK:` line. `ceiling` is the ceiling in force as text, or null for a gate
  whose only ceiling is the value the base holds. `id` is a hash of the gate name,
  the file and the declaration text, so it is the site identity of 4.4 in one
  token, and a harness can follow one finding across stops without parsing
  the rest. The id follows the path, so a file rename changes it while the
  site of 4.4 survives. Two findings at one site share the id, because 4.4
  keys a site by its file and its declaration text and pairs the findings
  there one to one. The list also carries the `error` and `unparsed` records
  of a run that could not measure something (14). Those name no site's
  values, so they carry no `id`, `ceiling` or `matched`, and their `outcome`
  says which kind each one is.
- `notes` entries of `{gate, outcome, file, text}`, plus the site's `line`
  and `values` where the note has them. `outcome` says which kind each one
  is: `unmatched` for an accepted entry that matched nothing, `unparsed` for
  a file a grammar refused in the hook, `lost` for a file `before` measured
  and `after` did not (8.6), and `note` for what a check left out of its
  count. `text` carries the reason, as the `NOTE:` line printed it.
- `exit` integer, the code the run returns. It is not read off
  `status`: a build failure that has spent its blocks is an `ERROR` run that
  exits 0, so a harness that wants the process's answer reads `exit` and one
  that wants the verdict reads `status`. Under `--hook`, 16.3 decides the
  stop's code after this object is built, so the journal line of 11.4 is the
  record that carries it in `exit`: 2 for a stop that blocks, 0 for one that
  passes.

A finding has no column, so the JSON carries none rather than a wrong one.

### 11.3 SARIF

`--sarif PATH` writes SARIF 2.1.0 to the file `PATH` names, with one rule per
gate and one result per failing finding (#65). Text output on stdout is
unchanged by the flag, which is why the log goes to a file and not to stdout
the way `--json` does. `--sarif` with `--json` is a usage error.

### 11.4 The journal record

One JSON line per event of 9.6. Every kind shares four fields:

- `schema` integer, 1 for this record. A reader MUST skip a line whose
  `schema` it does not know, and MUST count what it skipped so a report can
  say so. A schema bump without an upgrade in the reader MUST NOT ship.
- `version`, the klin version that wrote the line, and `time`, seconds since
  the epoch.
- `kind`, `"stop"`, `"prompt"`, `"guard"` or `"reset"`.
- `session`, the host's id for its grouping of turns, null where the event
  carried none or named no host, and always null on a `reset` line, which
  `klin turn reset` writes over no host event at all.

The `stop` line is the 11.2 object the stop's run built — the gates, a build
failure, or an error alike — plus what only the hook knew:

- `host`, the adapter's name, null where the event was unreadable.
- `prompt`, the counter of 6.2 the stop ran under.
- `hook` `{blocked, delivery, gate_spent, build_blocks, blocked_before}`.
  `blocked` is whether this stop exited 2. `delivery` is `block` or `none`;
  `follow-up` and `report` are reserved for a host whose stop cannot block
  (#67). `gate_spent` and `build_blocks` are the build stamp of 16.3 as this
  stop left it, and `blocked_before` is the host's flag.
- `verdict`, `green`, `red`, or `none` for a stop that wrote no verdict, with
  a `why` string beside `none` that names the reason this stop had and no
  other: the lock timed out, the state directory could not be readied, it held
  no stamp klin could read, or the stamp klin read could not be written back.
- `timing` `{total_ms, build_ms, lock_ms, klin_ms}`, where `klin_ms` is the
  total less the build, so the budget of 13 reads straight off it.
- `asked`, the site ids this stop asked about, as the turn stamp records
  them (8.2).
- `flags`, the unusual paths this stop took, empty on a clean stop:
  `turn-restored` (16.1), `branch-fallback` (a stop that judged a branch
  window because no stamp resolved), `count-unwritable` (a build stamp that
  would not write, 14).
- `told`, the parts of the `systemMessage` this stop printed for the person,
  empty where it printed none: `note` (8.2, 14), `turn` and `weekly` (9.5). A
  reader finds the last weekly line from it.
- `window`, the window the stop judged, which the line carries even where its
  run compared nothing against a base, so a reader knows which turn stamp
  each stop judged against (11.5).
- `config_hash`, a hash of the config file in force, so a later reader can
  tell a fix from a config change without a schema bump. Recorded and not
  read.

The `prompt` line is the `UserPromptSubmit` event `klin radius` ran on:

- `prompt`, the counter this event raised it to.
- `text`, the prompt's first line cut at 80 characters. Absent where
  `journal.prompt` (5.2) is `false`, where the configuration would not load,
  or where the event carried no prompt text.
- `radius` `{lines, formatting, moved, directories, wide}`, the facts `klin
  radius` measures (ADR 0014), present only where radius could measure them:
  a first prompt, with no mark yet to measure from, carries none. `wide` is
  whether the turn spread past the project's usual change, the same test
  that decides whether `klin radius` prints its note.

The `guard` line is one `ask` or one `deny`, never an `allow`: the guard
runs on every tool call under its 50 millisecond budget (13), and an allow
tells a reader nothing.

- `decision`, `"ask"` or `"deny"`: the answer the host delivered, not the one
  the guard reached. A host with no question to ask refuses instead (9.1), so
  an ask that reached the agent as a refusal is recorded as `"deny"`.
- `reason`, a hyphenated tag naming why the guard decided as it did:
  `config-write` and `state-write` for a proven write an edit tool's path or
  a redirect names, `config-mention` and `state-mention` for a command that
  only names a guarded path as a writer's argument, which klin cannot prove
  the way it proves the first two (ADR 0033), and `init` and `turn-reset`
  for one of klin's own subcommands that only a person runs.

The `reset` line is `klin turn reset`:

- `prompt`, the counter the stamp carried over. A reset does not end the
  turn, so the counter is unchanged by it (6.2).

The file is a public surface. `klin stats --json` (11.5) is what a harness
reads, and the two readers of the design read nothing else.

### 11.5 `klin stats`

`klin stats` reads the journal of 11.4 and reports what klin caught, for the
person and not for the agent. `--since Nd` sets the window, seven days by
default. `--all` lifts the cap of five items per group. `--json` prints the
episodes instead of the text. The command exits 0 whatever it finds: it
reports and judges nothing.

Two more scopes replace the window of days, and the three exclude each other:

- `--turn` reads the lines since the current turn stamp moved: after the last
  `reset` line, after the last green stop a `prompt` line followed, and after
  the last stop whose `window` names another stamp. A reset starts the report
  over, as it starts the judgment over (6.2).
- `--session` reads the lines carrying the newest `session` id the journal
  holds, and the `reset` lines among them, which carry none and still end the
  episodes before them.

The reader turns lines into episodes with no I/O and no clock of its own. An
**intervention** is one gate's failure on a stop that spent the prompt's gate
block, so one stop with three failing gates is one blocked stop and three
interventions. Its **outcome** is a relation between that line and the lines
after it, which the writer never stores:

- `fixed-next`, the gate ran and passed on the next stop
- `fixed-later`, the gate ran and passed on a later stop in the window
- `asked-once`, a later stop recorded the site as one klin let through after
  it asked (8.2), so the code is as the agent left it and the fix is the
  person's to make
- `reset`, a person moved the turn stamp before the gate went clear
- `open`, no stop in the window ran the gate and passed it

A gate the stop's `gates` list of 11.2 carries no row for did not run, and a
row that says `ERR` measured nothing. Neither ends an episode, because neither
says the site went.

Nothing in the reader names a check. A line for a gate the binary has no
check for is read and printed like any other.

`--json` prints one object: `window` `{scope, days}`, where `scope` is
`turn`, `session` or `days` and `days` stands only beside `days`, `stops`, `skipped` (the lines
of 11.4 the reader could not read or does not know), `unreadable` (how many distinct
files the window's stops could not read or measure, counted once each), `counts` `{caught, fixed-next,
fixed-later, reset, open, asked-once}`, `episodes`, `asked` and `earlier`.
`episodes` is a list of `{gate, file, line, text, remedy, time, more, outcome,
prompt}` newest first, where `more` is how many further findings that gate
left on that stop and `prompt` is the excerpt of 11.4 the stop ran under, or
null. `asked` is a list of `{time, kind, decision, reason, file, line}` newest
first, one for each item of `You were asked`: `kind` is `guard`, `reset` or
`asked-once`, and a field that does not apply to the kind is null. `earlier`
is `{caught, open}` for the window before this one, or null where the journal
does not reach back over it. The object holds facts and none of the person's
sentences.

The text has these line shapes:

- the title, `klin, SCOPE in PLACE`, where SCOPE is `this turn`, `this
  session`, `today`, `this week`, `this month` or `the last N days`, and PLACE
  is `this repository` where the repository has one worktree and `this
  worktree` where it has more
- the headline, `klin caught N shortcuts.`, and beside it `The agent fixed N
  of them on its own and asked you N times.` The fixed half reads `fixed it`,
  `fixed both` or `fixed all N` where the agent fixed every one, and `asked
  you once` for one. Each half is left out where its count is zero, so the
  sentence can read `The agent asked you once.` The asked count is the items
  of `You were asked`. Then `One is still there.` or `N are still there.` on
  its own line
- the groups `Still there`, `Fixed after klin asked` and `You were asked`, in
  that order, and no heading carries a count. An open item reads `SITE in
  FILE:LINE, left on DAY` and carries its check's remedy on the line under it;
  every other group is headed by the day it happened on: `Today`,
  `Yesterday`, the weekday name inside seven days, then `YYYY-MM-DD`. The
  local offset is read from the system once per report, with UTC as the
  fallback.
- an open item, a fixed item and a shortcut a reset set aside end with `,
  while you asked for "EXCERPT"` where the stop's prompt line of 11.4 carries
  an excerpt, quoted as the person wrote it. The item carries nothing in its
  place where `journal.prompt` is `false` or the excerpt is not in the lines
  read
- `You were asked` holds one item per guard answer, reset and `asked-once`
  episode, newest first:
  - `klin asked before X` for a guard `ask` and `klin refused X` for a guard
    `deny`, where X names the reason tag of 11.4 (`an edit to klin.json`, `a
    command that named klin.json`, `an edit to klin's own state`, `a command
    that named klin's own state`, `klin init, which only you run`, `klin turn
    reset, which only you run`, and `a tool call` for a tag the binary does
    not know)
  - for a reset that ended episodes, `You set aside N shortcuts.`, each
    shortcut on its own line under it, and then ``If they're still there, fix
    them, or accept them in `klin.json`, before you push.``, in the singular
    for one. The report cannot see whether the code is still in the tree, and
    CI still judges the branch (6.3). A reset that ended none reads `You told
    klin to start over.`
  - `a test deleted from FILE:LINE, SITE. The agent said why.` for an
    `asked-once` episode, where SITE is the declaration line without a
    trailing `{` or `:`, and `FILE deleted. The agent said why.` where the
    site names no declaration because the whole file went
- `and N more. klin stats --all` under a group the cap trimmed
- `Last week: N shortcuts, N left open. This week is better.`, for a window of
  days where the journal reaches back over the whole window before it, and
  never otherwise. The word is `better`, `worse` or `the same`, judged on the
  open count first and on the caught count after, and only this worktree's
  journal is read. The two names follow the title: `Yesterday` and `Today`,
  `Last week` and `This week`, `Last month` and `This month`, or `The N days
  before` and `These N days`
- `klin ran N times and took N seconds in total.`
- `Measurement`, with what klin skipped or could not read under it, printed
  only where there was something. Green with half the tree unparsed is the
  one lie the report must not tell.
- `klin caught no shortcuts. klin ran N times and asked nothing.` for a
  window with no intervention, with `asked you N times` in place of `asked
  nothing` where `You were asked` holds items, and `klin started watching today. Come back
  after a few turns.` for a journal with no line at all.

No word of the agent's glossary appears in the text, and the report carries
no score, no color, no glyph, no praise and no estimate of time saved.

## 12. Determinism

- Every `git diff` klin runs MUST pin `--diff-algorithm=histogram` and pass
  `-M` or `--no-renames` by name (ADR 0014).
- Findings MUST be sorted by file then line before matching and before
  printing.
- Grammars are compiled into the binary. A grammar version change is a klin
  version change, and the survey cache key includes the version.
- No check MAY read the network.
- The only clock a judgment reads is a pinned dated ceiling (5.5), read in
  UTC, and `KLIN_TODAY` overrides it. The report age check of 8.3 compares
  file times and is the one other place time enters a verdict. The `ms` of
  11.2 and the `time` and `timing` of 11.4 are measurements about the run,
  recorded and never judged, so they do not break determinism: every field a
  verdict depends on is still a pure function of the trees.
- A `run` entry in 8.3 is deterministic only when the tool it runs is. klin
  MUST record the command it ran beside the results.
- A derived number is a pure function of the derivation commit and the binary
  version. A derived path set is the union of that commit's survey and the
  `after` tree (4.3).

## 13. Performance Budget

The Stop hook, excluding the project's own build, SHOULD finish within 5
seconds on a tree of 2,000 source files when scoped with `--changed` to 20
files and the survey cache is warm. The first stop on a new base MAY take the
whole-tree survey and SHOULD finish within 30 seconds on the same tree. A
whole-tree `--strict` run in CI SHOULD finish within 60 seconds. An
implementation MUST measure all three on a fixture and record the numbers in
the release notes when they move by more than a third.

The guard MUST finish within 50 milliseconds.

The normative measurement fixture is the ignored `performance_fixture` CLI
test, selected by `cargo test -- --ignored perf`. It generates a deterministic
temporary repository with `rust/` and `web/` projects: the 2k row holds 1,000
`.rs` and 1,000 `.ts`/`.tsx` files, and the 10k row holds 5,000 of each. A
small one-percent TypeScript subset is `.tsx`; both rows include Rust and
TypeScript manifests, tests, imports and references, and held escape/stub
sites. Each row changes 10 files in each language. The test runs warm hook,
cold survey and whole-tree strict five times and prints the median; the hook
row explicitly excludes the project build. It also sends 1,000 deterministic
read, write, shell, quoted-path, glob, heredoc, configuration-name and
multi-path patch events through the real guard binary path five times, as a
separate row. The test records the klin version, fixture counts, cache state,
changed-file counts, iteration count and median milliseconds; it does not
enforce the budgets on contributor hardware.

A check that cannot take scope, such as a whole-tree duplication share, MUST
say so in `gate --list` and MAY be skipped by the hook under a `hook: false`
key on its section.

## 14. Failure Model

Two flags change the failure model, and nothing else does. `--hook` turns a
failure the agent cannot fix into a NOTE, or sends it to stderr with exit 1,
so a stop is never blocked on it. `--strict` turns a hole a person's CI must
not miss into exit 2. `klin gate` with neither flag exits 2 on every hole ADR
0003 named and passes with a NOTE on the holes that `--strict` adds (10).
klin cannot see CI, so no row says "CI". A row that names no mode behaves the
same in all three.

| Class | Behavior |
|---|---|
| Config error: unknown key, malformed section, schedule with no due step | exit 2 before any gate runs, naming the file and key. Hook: the same text on stderr with exit 1, never a block, because the agent cannot edit the file it names (9.4) |
| No base resolves outside the hook | exit 2 naming what was tried |
| `turn` file missing in the hook, ref present | restored from the ref with a RED verdict, and a NOTE says so |
| `turn` file and ref both missing in the hook | a branch window from the base of 6.3, or from HEAD when none resolves, a NOTE names the missing stamp, and the stop writes that base as the stamp |
| A file no grammar reads | Outside the hook: the gate names it and exits 2, other findings still print. Hook: a NOTE, told to the person through `systemMessage` on a stop that ends (9.1). |
| A deleted test (8.2) | Hook: blocks the first stop that finds it, once. The next stop lets it through as a NOTE, tells the person, and ends green. Outside the hook: a NOTE, `--strict` included. |
| The build fails in the hook | block with the build output, no gate runs. Outside the hook the build step does not run (ADR 0012). |
| Host event unreadable in the hook | report to stderr and exit 1, never block |
| Host event unreadable in the guard | allow |
| A `run` entry exits without writing its report, or the report is not SARIF | that gate is ERR, in every mode. The command's exit status alone is not judged (8.3). |
| Survey cache unreadable | recompute, overwrite |
| State directory unwritable | the hook reads the stamp it can find, per the two rows above, writes no verdict and no build count, prints why, and never blocks on it. A build failure is reported, not blocked, because no count could bound the blocks. |
| Survey finds no source root | `--strict`: exit 2 naming the directory surveyed. Otherwise a NOTE naming it, and in the hook the turn ends. |

## 15. Trust Model and Conformance Levels

### 15.1 Feedback level

Hooks only. klin puts every failure in front of the agent once per turn, keeps
the window open until the failure is fixed, accepted or reset by a person,
and refuses its edits to the config. The config and klin's own state
directory are the guarded set (9.4).
Nothing prevents a PATH
shim or a `chmod -x`, and the guard sees only the tool calls the host shows
it. This level is what a person gets with no CI, and this document makes no
stronger claim for it.

### 15.2 Enforced level

Feedback level plus: a CI run with `--strict`, on a checkout the agent never
touched, against a protected branch, with `klin.json`, the workflow, the hook
settings and CODEOWNERS under CODEOWNERS. At this level a gate holds against
an agent, and loosening it takes a reviewed commit by a person.

One finding is the exception. A deleted test is a NOTE in CI (8.2), so at
this level it holds only through the hook's one question, the guard in front
of the record that question leaves (9.4), and the reviewer who reads the
diff. An agent that runs with no hook meets none of the three. ADR 0031 and
ADR 0032 record the cost.

The README MUST name both levels and say which one a setup reaches.

### 15.3 Secrets and safety

klin reads no secrets and sends nothing anywhere. The `build` and `run`
commands execute a project's own shell lines, which the config owner wrote,
under the guard. A derived `build` command is one of a fixed table keyed by
manifest, printed before it runs.

## 16. Reference Algorithms

### 16.1 The hook's window

```
state = KLIN_STATE_DIR/hash(common_dir, worktree) if set else git_dir()/klin

read_stamp():
  stamp = read(state/turn)
  if stamp is not None: return stamp
  commit = resolve("refs/worktree/klin/turn")
  if commit is None: return None
  note("turn file missing, restored from the ref")
  stamp = Stamp(commit, parent=parent(commit), time=None, last_verdict=RED)
  write_atomic(state/turn, stamp)
  return stamp

hook_window():
  stamp = read_stamp()
  if stamp is None:
    note("stamp deleted, judging the branch")
    before = choose_window(strict=False).before or HEAD     # 16.2
    write_atomic(state/turn, commit=before, parent=before, time=now, last_verdict=RED)
    return Window(BRANCH, before, WORKING, "stamp missing")
  return Window(TURN, stamp.commit, WORKING, "since " + stamp.time)

on_session_start_or_prompt():          # one rule for both events
  stamp = read_stamp()
  tree  = write_tree(index=state/index, add_all=True)
  if event == PROMPT: report_radius(read_mark(), tree)   # 6.2.1, never on a session start
  if stamp is None and not exists(state): move_stamp(tree)   # first session here
  elif stamp is not None and stamp.last_verdict == GREEN: move_stamp(tree)
  elif stamp is None: note("stamp deleted, the next stop judges the branch")
  move_mark(tree)                                  # 6.2.1, on both events, whatever the verdict
  if exists(state/turn): bump_prompt(state/turn)   # prompt += 1, whether or not the stamp moved

turn_reset():                          # a person's command, denied by the guard
  tree = write_tree(index=state/index, add_all=True)
  move_stamp(tree); move_mark(tree); print("a person moved the window")

move_stamp(tree):
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/turn", commit)
  write_atomic(state/turn, commit=commit, parent=HEAD, time=now, last_verdict=None,
               prompt=current_prompt(state/turn), mark=current_mark(state/turn))

move_mark(tree):                       # 6.2.1, the window the radius report measures
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/mark", commit)
  write_atomic(state/turn, mark=commit)

read_mark():
  return mark_of(state/turn) or resolve("refs/worktree/klin/mark")

derivation_commit(window):
  return stamp.parent if window.kind == TURN else window.before

roots(window):
  return survey(derivation_commit(window)).roots | survey_roots(window.after)
```

### 16.2 `klin gate` and CI window

```
choose_window(strict):
  for (ref, kind, source) in candidates():
    before = resolve(ref)
    if before is None: continue
    if before == HEAD and tree_is_clean():
      if source == Remote: return Window(kind, before, WORKING, "same, remote agrees")
      tip = remote_default_tip()
      if tip is None:
        if strict: fail("cannot tell whether history is hidden")
        return Window(kind, before, WORKING, "same, no remote to ask")
      if commits_between(tip, HEAD) > 0: fail("unpushed commits hidden")
      return Window(kind, before, WORKING, "same, remote holds HEAD")
    return Window(kind, before, WORKING)
  fail("no base resolves")
```

### 16.3 The hook run

```
hook(event):
  host = Host.detect(event)
  window = hook_window()
  at = derivation_commit(window)
  survey = cached_survey(at) or survey(at)
  count = read(state/build-blocked)
  if count is None or count.prompt != turn.prompt:
    count = Count(prompt=turn.prompt, builds=0, gate_spent=False)   # a new turn
  failure = build(config_or(survey), changed_files(window))
  if failure:
    count.builds += 1; write_atomic(state/build-blocked, count)
    write_verdict_atomic(state/turn, RED)
    if count.builds > 8: report(failure, "stopped blocking after eight"); return 0
    block(failure)
  (failed, errored, reported, told) = run_gates(config_or(survey), window, scope=changed)
  if failed == 0 and errored == 0:
    write_verdict_atomic(state/turn, GREEN)
    if told: tell(report)                          # systemMessage on stdout, exit 0 (9.1)
    return 0
  write_verdict_atomic(state/turn, RED)
  if count.gate_spent: report(); return 0
  if count.builds == 0 and host.blocked_before(event): report(); return 0
  count.gate_spent = True; write_atomic(state/build-blocked, count)
  add_asked_atomic(state/turn, reported)           # 8.2, cleared when the stamp moves
  block(report)
```

`reported` is the site id (11.2) of every finding the run printed, and `told`
counts the notes a person needs even when nothing blocks: a file no grammar
read, a file measured in `before` only, and a deleted test the run let
through. Only a stop that blocks on a gate adds to `asked`, so a question the
agent never saw is asked again at the next stop that blocks. `inventory`
reads `asked` under `--hook` (8.2).

The build stamp is one record per prompt: the prompt counter it belongs to,
the number of build blocks, and whether the turn's one gate block is spent.
A passing build does not reset the build count, so a tree that builds, breaks
and builds again inside one turn still gets eight blocks in that turn and no
more. The host's `blocked_before` flag is a second opinion for the first gate
block only, because after a build block that flag is true while the gate
block is still unspent (ADR 0004).

### 16.4 Evaluate one gate

```
evaluate(check, section, window, scope):
  before = check.measure(window.before, section) if check.needs is the tree else []
  after  = check.measure(window.after,  section, before)   # only inventory reads `before`
  entries = accepted(config, check.name) + before   # accepted first: entry order breaks ties (16.5)
  (after, entries) = restrict(after, entries, scope)
  ceiling = section.ceiling            # derived from the derivation commit, pinned, dated, or none

  after = [f for f in after if ceiling is None or over(f, ceiling)]
  before_over = [e for e in entries if ceiling is None or over(e, ceiling) or e.accepted]
  return judge(after, before_over, check.ratcheted)
```

`before_over` is what makes 7.3 hold: an entry below today's ceiling is not
judged, and an entry above it is matched, so a lower ceiling never turns a
held site red. A check with no ceiling, such as `inventory`, judges every
finding and every entry, so its `missing: 0` entries stay in and a vanished
site matches its `before` entry.


`inventory` fits the same judge by ratcheting existence. Its measure of
`after` emits a finding for every site the `before` measure holds, with
`missing: 1` where `after` has no match by site and then by body hash, and
`missing: 0` where it has one. The `before` entries carry `missing: 0`, so a
vanished site is `worsened` under the one `judge` and nothing else changes.
A vanished site the run lets through, outside the hook or in a stop's
`asked` record (8.2, 16.3), has a `before` entry of `missing: 1` instead, so
the same judge holds it and the run lists it in a NOTE. A site the accepted
list names is matched like any other, ahead of the `before` entry by entry
order. A vanished site whose subject went in the same window is a NOTE, not
a finding.

### 16.5 Match one site

```
match_site(findings, entries, ratcheted):
  # findings are sorted by line; entries are the accepted list in config
  # order, then the `before` sites by line
  candidates = []
  for i, f in enumerate(findings):
    for j, e in enumerate(entries):
      rose     = any(f[m] > e[m] for m in ratcheted)
      shared   = count(m for m in ratcheted if e[m] == f[m])
      distance = 0 if e.accepted else abs(e.line - f.line)
      candidates.append((rose, -shared, distance, i, j))
  pairs = []
  for (_, _, _, i, j) in sorted(candidates):
    if taken(findings[i]) or taken(entries[j]): continue
    take(findings[i]); take(entries[j])
    pairs.append((findings[i], entries[j]))
  return pairs, untaken(findings), untaken(entries)
```

The judge runs this once per file and declaration text (4.4). An untaken
finding is `new`. An untaken `before` entry has no outcome. An untaken
accepted entry matched nothing (4.8). The second pass of 4.4 takes the
untaken findings and entries as its input, grouped by body hash rather than
by site, with `distance` fixed at 0. Each group sorts both sides by file and
line before it runs, so what is left of the rank order, `i` and `j`, is the
finding's file and line, then the entry's. Only an entry whose site the
`after` tree lost reaches this input (4.4).

Inserted twin. `before` holds `fn f() {}` at lines 1 and 5. `after` holds it
at lines 1, 3 and 6, all with equal values. Every pair shares every value,
so distance decides: line 1 takes line 1 at distance 0, then line 6 takes
line 5 at distance 1. Line 3 is untaken and `new`. Pairing by line order
would pair line 3 with line 5 and name line 6, which nobody wrote, as new.

Moved twin. `before` holds `fn twin()` with cc 2 at line 3 and cc 1 at line
20. `after` holds cc 2 at lines 19 and 50. Both findings share both values
with the cc 2 entry. Line 19 is nearer, so it takes that entry and is held.
Line 50 takes the cc 1 entry and is `worsened`, cc 1 to 2. Pairing by
nearest line would pair 19 with the cc 1 entry at 20 and report a function
that only moved as worse.

Stale accepted entry. `before` holds `fn f()` with cc 3 and 5 lines. The
accepted list holds it at cc 2 and 4 lines. `after` holds it at cc 2 and 5
lines. The finding shares one value with each entry. It holds against the
`before` entry and rose against the accepted one, so the `before` entry takes
the match and the site is held (7.3). The accepted entry matched nothing.
Without the first rank, entry order would give the match to the accepted
entry and report a site that got no worse as `worsened`.

## 17. Test and Validation Matrix

One seam, the binary, on a throwaway tree with a base (AGENTS.md).

Core:

- Config: absent file runs green, discovery, override, relative paths,
  unknown key, `baseline` key refused, `false` exclusion, pinned beats
  derived, dated ceiling picks the right step, schedule with no due step is
  an error, old key `sources` names the new one.
- Survey: roots on a single project, on a monorepo, on a tree with no source,
  derived ceilings on a tree with fewer than 50 functions, cache hit and miss,
  cache keyed by binary version.
- Window: each candidate in order, each ADR 0013 branch outside the hook, the
  hook with a deleted `turn` file restores it from the ref with a red
  verdict, the hook with file and ref both deleted judges the branch and the
  prompt before it writes no fresh stamp, the stamp holding through a commit, the
  derived values holding through a commit inside a turn, the stamp not moving
  after a red stop on a prompt and on a session start, accepting finding A
  holds A and leaves B failing and the stamp where it was, `turn reset`
  moving it and saying so, the stamp surviving `git gc`, a hook killed
  mid-write leaving the previous `turn` file intact, the survey cache hitting
  on the second stop of a turn and missing after a commit.
- Ratchet: new fails, worsened fails, held passes, rename keeps sites, moved
  function keeps its site within a file and across files, a function renamed
  with its body unchanged keeps its site, a one-line function that moved
  keeps its site, a moved body that was reindented keeps its site, a moved
  body that was edited is `new`, two bodies that moved take the entry that
  shares their values, the lower of two bodies in one file takes the moved
  entry, an accepted entry beside the base entry does not free it for a
  copy, a function that moved and grew names the site it matched, a copy of
  a function beside its original is `new` and does not inherit the
  original's match, a twin inserted between two twins is the new one and its
  neighbours hold, a moved twin keeps its entry over a nearer twin whose
  value changed (16.5), a lowered ceiling fails no held site, accepted entry
  holds a site, an accepted entry the base also holds at the same values
  stays matched under `--strict`, a stale accepted entry does not fail a
  site the base holds, an accepted entry that names some of the values, or
  gives one a value that is not a number, is exit 2, unmatched accepted
  entry is a NOTE and a strict failure, a failure prints the site it matched
  and both values.
- Measurement rules (8.2.1): a document whose words are split by Unicode
  spaces and hold markup is counted at the ceiling boundary, a byte that is
  not UTF-8 is one word, a repeated line of two escape kinds fails as one site
  with the first label and every match counted, a backticked span that is not
  a path is not read, a nested function is measured on its own.
- Each check: over, at, under the ceiling, a file it cannot read with
  `--hook`, with neither flag and with `--strict`, of which the first is a
  NOTE and the other two exit 2, scope restricts both sides, `--json` shape
  with coverage counts,

  a tree with no config and existing debt of this check's kind is green, a
  new root added in the window is measured, a first file in a new language
  is held to the floor, a new document is a NOTE with its word count and is
  judged once the stamp moves, a directory that becomes a root in the window
  brings no held sites with it, and a function under an `after`-only root
  does not move the derived ceiling.

- `inventory`: a deleted test file whose subject was deleted too is a NOTE, a
  deleted test function or file blocks the first hook stop that finds it and
  asks why it went, the next stop passes and tells the person which tests
  went, a prompt between the two stops does not ask again, a stop whose
  stamp was deleted still asks, outside the hook a deletion is a NOTE under
  `--strict` too, an accepted entry naming a deleted test matches, a renamed
  test function with its body unchanged is held, a deleted test function
  whose file went too is a NOTE.
- `sarif`: a result on a changed line fails, on an unchanged line in a changed
  file is held, `differential` fails every result, `run` writes the report
  before it is read, a report older than a changed file is ERR.
- `lockfile`: a manifest entry with no lockfile entry fails, a removed pin
  fails, a path dependency is not judged, a new dependency with a range and a
  lockfile entry is green, a deleted lockfile fails every dependency, a
  workspace lockfile above the member manifest is found, both npm lockfile
  versions are read, an unreadable format is a NOTE, a malformed supported
  lockfile is a tool error, a derived manifest klin cannot parse at either
  commit is a NOTE, a derived manifest the work broke is a tool error, and a
  pinned one is a tool error.
- Coverage: a file present in both trees and measured in `before` only is a
  NOTE in the hook and exit 2 under `--strict`, whether it left through an
  exclusion or a grammar error.
- Concurrency and worktrees: an old green stop that finishes after a new red
  stop does not write green, two worktrees keep independent stamps through
  `git gc --prune=now`, a stop that cannot take the lock writes no verdict.
- Guard, the files that left the guarded set: an edit to a host's hook file,
  CODEOWNERS, a lint config, a test config, a coverage threshold, a CI
  workflow and klin's state directory is allowed.

Paired scenarios. Every `stubs`, `escapes` and `inventory` pattern carries
two fixtures, one shortcut that fails and one legitimate change that stays
green, because deterministic detection is not correct judgement:

- A failing test is deleted, fails. The same test moves to another file with
  its body unchanged, green.
- A test body is emptied, fails. A test body is rewritten with the same
  declaration, green.
- A `pass` body lands on a function, fails. A `pass` body lands on an
  exception class, green.
- A new root or an exclusion changes what is measured, and the coverage line
  says so on the same run.
- A finding survives a commit, a new prompt and a new session, and its `id`
  is the same in each JSON report.
- Accepting finding A holds A and leaves B failing.
- A `run` command that exits without writing the report is ERR, and a run
  that writes it is judged on the new report.
- Runner: cheapest first, every gate runs after a failure, ERR beats FAIL in
  the exit code, `--gate` on an excluded gate, `--list` shows derived and
  pinned, no source root is exit 2 under `--strict` and a NOTE in the hook.
- Hook: build failure blocks every stop and stops after eight, a new prompt
  restores the eight, a build failure writes a red verdict and the next
  prompt does not move the stamp, gate failure blocks once, the stamp hands
  the second stop an unspent block, a second session's prompt in the same
  worktree does not spend it, unreadable event never blocks, the verdict is
  written.

- Guard: one test per deny route including `init` and `turn reset`, one per
  ask route, every reader allowed including `git rev-parse` and `git cat-file`
  on the ref, every file that left the guarded set allowed for both an edit
  and a write, a heredoc opened inside a command substitution allowed, glob
  does not match by empty prefix, quoted pipe does not split, under 50
  milliseconds.
- Init: pins exactly what the run would derive, writes only the config,
  `--add` leaves `false` alone, `--force` re-pins, never touches
  `.gitignore`, `--hooks` writes each host's file and leaves an existing
  entry alone, `--hooks --global` writes the user-level file and leaves the
  tree's own untouched, a written line exits 0 when no binary resolves, and a
  host whose plugin is enabled gets no entries at all.
- State: default under the git directory, per worktree, `KLIN_STATE_DIR`
  relocates it, two clones never share a stamp, an unwritable directory never
  blocks, `cache clean` removes only klin's files.
- Install: the plugin wrapper fetches the pinned version once and verifies the
  checksum, a second run makes no network call, a missing network on first run
  lets the turn end with a message.

Extension:

- Fixtures with hand-checked values for complexity per language (ADR 0001).
- A `run` sarif entry over a tool that needs no dependencies.
- The performance fixture of section 13.

## 18. Implementation Checklist

Core, in this order:

- [ ] Guard: fix the glob prefix and quoted splitting, add the `ask` decision
- [x] State directory under the git directory, `KLIN_STATE_DIR` override,
      `cache clean`
- [ ] The turn stamp as a commit with HEAD as parent, under `refs/worktree/klin/turn`
- [x] Amend ADR 0016 and 0017 to match 5.4, 6.2 and 6.6
- [x] Amend ADR 0016, 0020 and 0022 for the fourth review of section 0

- [ ] One stamp rule for session start and prompt, `turn reset` for a person,
      the `turn` file written atomically with its prompt counter, a lock on
      the state directory for the whole stop, the build stamp counting blocks
      under the prompt counter and stopping after eight, a red verdict before
      a build block

- [ ] The hook reads the turn window, writes the verdict, restores a missing
      `turn` file from the ref, and judges the branch when both are gone
- [ ] The guard denies `init` and `turn reset`, asks on a non-reader that
      names `klin.json`, and allows every file that left the guarded set

- [ ] Survey at run time from the derivation commit, cached by it, derived
      values printed. Derived numbers come from the derivation commit's own
      paths. Roots, languages, documents and manifests are the union of that
      survey and the `after` walk, and a site under a path the survey did not
      hold is `new`.
- [ ] `doc-citations` and every other derivable check compare to `before`
- [x] No source root is exit 2 under `--strict`, and `--list` says derived or
      pinned per key
- [ ] Derived complexity ceilings, floor and minimum sample, floor for a new
      language
- [ ] Pinned dated ceilings in every check that takes a ceiling, in UTC, date
      printed
- [x] One key vocabulary, old names print the new one, differential test
      retired
- [x] Unreadable file is a NOTE in the hook
- [ ] Coverage counts on every `OK:` line, matched site and both values on
      every failure, a finding `id`, all in the JSON, and the coverage
      regression NOTE and strict failure
- [ ] Host adapter, Claude Code first, README stops naming other hosts
- [ ] Three escapes rows: `fit(`, `fdescribe(`, `xfail`
- [x] Cross-file move matching by body hash (4.4)
- [x] `inventory` over test files and test functions, with the
      deleted-subject NOTE and the body-hash rename match
- [x] `stubs`, sharing the escapes engine, executable function bodies only,
      the empty test body included, with a legitimate-change fixture per row
- [x] `sarif`, `after` only, delete `report`, `run`, read `report`, scoped to
      changed lines
- [x] `lockfile`, Rust, npm and Go, with pnpm, yarn, Poetry and uv deferred
- [ ] `CONTEXT.md` takes Window and Derived, README names the two
      conformance levels
- [ ] New ADRs for each row of section 0 that is accepted, and one for the
      stamp as a parented, ref-held commit

Next, after core is green, each with an unsolved problem named in section 8:

- [ ] `duplication`
- [ ] `sarif` with `compare`, once the dependency problem of 8.3 has a design

Distribution, in this order, because each step depends on the one before:

- [x] Release pipeline: four binaries and a checksum file per tag (#62)
- [ ] Install script with `--version`
- [x] The Claude Code plugin with `hooks.json` and the `bin/klin` wrapper (#66)
- [x] `init --hooks` for Codex and its host adapter (#68)
- [ ] `init --hooks` for Cursor and its host adapter (#67)
- [ ] The GitHub Action
- [ ] Homebrew tap, `cargo install`, npm wrapper (#64)

Recommended:

- [x] Configuration reference (#18) generated from each check's declared keys
      and derivation rules, section 5.8
- [ ] `--sarif` output

Before calling it 1.0:

- [ ] Performance numbers from section 13 recorded on a fixture
- [ ] A task comparison with and without klin on a small set of agent tasks,
      recording regressions caught, legitimate changes blocked, extra repair
      turns and hook latency. This is a benchmark, not a test, and it is what
      shows the tool is useful rather than correct.
- [ ] The hook-output facts in 9.3 verified against the host's documentation
- [ ] Cursor adapter, or the README stays silent on it

## 19. Installation and Distribution

Two things are installed, and they stay separate. The binary is one static
file per platform. The hooks are three lines of host configuration that call
it. Every route installs the binary once and then writes hook lines.

### 19.1 The binary

A pushed tag `vX.Y.Z` builds the binary for macOS and Linux, on x86_64 and
arm64, and attaches the four archives, a `.sha256` beside each one, a
`sha256.sum` over all of them, and the install script to a GitHub release.
`dist` runs that pipeline, so its artifact names and its install script are
what a route consumes. ADR 0026 records that choice. `klin --version` prints
the version the binary was built from, which is the tag without its `v`. Every route below
downloads from that release and MUST verify the checksum. The routes, in
order of least friction for the person:

1. The install script the release carries, `klin-installer.sh`, run through
   `sh`. It detects the platform, verifies the checksum it was generated
   with, and puts `klin` and `klin-update` in `~/.local/bin`, or in the
   directory `KLIN_INSTALL_DIR` names. Each release carries the script that installs
   that release, so a URL under a tag pins a version.
2. A Homebrew tap, for macOS and Linux users who already have brew.
3. `npm install --save-dev klin`, for a JavaScript project. The package holds
   no compiled code. Its install step downloads the release for the host and
   verifies it. The package version equals the release tag, so the lockfile
   pins the binary.
4. `cargo install klin`, for a Rust user. Free once the crate is published.
5. A GitHub Action, `action.yml` at the root of this repository, that
   installs a pinned version and runs `klin gate --strict`. For CI only. A
   workflow pins it by the release tag, `uses: brajevicm/klin@vX.Y.Z`, so one
   tag names the binary, the plugin and the Action.

Cross-compilation for Windows is not a target of this draft. The Codex hook
system is not available on Windows either.

### 19.2 Claude Code and Codex CLI: the plugin is the whole install

One plugin directory serves both hosts. It holds `hooks.json` with the three
hooks of 9.2, one skill that tells the agent how to read a failure and what
it may not touch, two slash commands that run the gates and list them, and a
`bin/klin` wrapper. The manifest names the hooks file, so neither host has to
find it by convention.

Codex CLI reads the same manifest and the same `hooks.json` shape. It
substitutes the literal `${CLAUDE_PLUGIN_ROOT}` into a plugin's hook line,
and the hook lines do not rely on the variable being exported, because a
shell default form such as `${CLAUDE_PLUGIN_ROOT:-}` was left unsubstituted
and expanded to nothing. Claude Code exports the variable and reads the bare
form the same way, so the hook lines name the plugin root in that form and
no other, and the same lines run on both hosts. It finds the plugin through a marketplace
file of its own at `.agents/plugins/marketplace.json`, which points at the
same directory as Claude Code's `.claude-plugin/marketplace.json`. The
install is `codex plugin marketplace add brajevicm/klin` and
`codex plugin add klin@klin`, and Codex asks the person to review the
plugin's hooks once before they run. The pre-tool matcher names
`apply_patch` beside Claude Code's edit tools, so the guard reads Codex's
edits. ADR 0030 records the decision. The Codex IDE extension loads no
plugins, so it takes the route of 19.3.

The wrapper is a shell script. It reads the version from the plugin manifest
beside it, so the plugin carries one pin. On first run it downloads that
release into `~/.cache/klin/bin/<version>/klin`, verifies the
checksum, and executes it. That install removes every other version from the
cache, so the cache holds one binary. Every later run executes the cached
binary with no network call. When the download fails, the wrapper runs a
`klin` that PATH resolves if there is one, and otherwise prints one line
saying so and exits 0, so a turn is never blocked by a missing network.
This is the one place klin touches the network, and it is install, not
measurement.

Every line the wrapper or a hook prints on exit 0 is a JSON object with a
`systemMessage`, because that is the one shape both hosts show as a notice on
every event. Codex rejects plain text on a Stop that exits 0, and Claude Code
writes it to the debug log alone.

Each hook line runs `${CLAUDE_PLUGIN_ROOT}/bin/klin` when that file is
executable, and otherwise the `klin` that PATH resolves. Claude Code appends
every installed plugin's `bin/` to the end of PATH, for hooks as for the Bash
tool, so a binary an installer left in `~/.local/bin` would win over the
wrapper if the hooks called `klin` by name. Calling the wrapper by its path
makes the version the plugin pins the one Claude Code runs. Where `bin/` is
unavailable, which is the case for plugins distributed through organization
settings, the hooks find `klin` on PATH from a route in 19.1, and the skill
says which command installs it. When neither is present the Stop hook says so
once and lets the turn end.

The hook lines resolve the binary before they run it, because a person may
install the plugin where no binary resolves yet. With neither the wrapper nor
a `klin` on PATH the session start, the prompt and the pre-tool events say
nothing and block nothing. The Stop hook names the install command in a
`systemMessage` on stdout, and only where a `klin.json` resolves at the
project root, so a tree that never opted in stays silent (ADR 0028). Nothing blocks, so the turn ends at that stop and
the line appears once.

The wrapper reads two overrides, `KLIN_RELEASE_BASE_URL` and
`KLIN_CACHE_DIR`. They exist so a CLI test can fetch a release of its own over
`file://` and prove the two paths that a real release cannot: the first run
that installs, and the failure that installs nothing.

With the optional config of section 5, installing the plugin is the complete
install. No `init` runs. The first stop is gated.

This reverses ADR 0002. Its first reason, a version pin beside committed
baselines, went with ADR 0009. Its second reason is handled by the PATH
fallback above. A version difference between the wrapper's binary and a CI
binary is a NOTE per 5.2, not a failure.

### 19.3 Cursor and Codex: a hooks file in the repository

Both hosts are shell-hook hosts (9.1). Cursor has no plugin of klin's, and a
Codex team may prefer hooks that are committed and covered by CODEOWNERS over
the plugin of 19.2. On this route the binary comes from 19.1 and
`klin init --hooks` writes the host file:

- Cursor reads `.cursor/hooks.json` at the project root, and its blocking
  events answer `allow`, `ask` or `deny`, which is the guard's vocabulary.
  `beforeShellExecution` and the file-edit events carry the guard.
  `beforeSubmitPrompt` carries `radius`. `stop` carries the gate.
- Codex CLI reads a `hooks.json` with `PreToolUse`, `UserPromptSubmit` and
  `Stop` at turn scope. The mapping is one to one with Claude Code's.

`init --hooks` detects a host by the presence of `.claude/`, `.cursor/` or
`.codex/` at the root, or takes `--host NAME`. It adds klin's entries and
leaves every other entry alone. It MUST NOT overwrite an entry that already
calls `klin`. The files are committed, so a teammate who clones gets the
hooks, and CODEOWNERS SHOULD cover them. The hook file is not guarded (9.4),
but `init` in any form is refused from an agent, so this command is too.

Each line klin writes resolves `klin` on PATH before it runs it and ends the
hook when none resolves, the way the plugin's own lines do (19.2). A person
who never installed the binary, or who removed it, sees nothing rather than a
failed hook on every event.

`init --hooks` writes nothing for a host that already runs klin's hooks over
the file it would write, and names what runs them. Two things do: a plugin,
and a user-level install klin wrote itself, which a host reads together with
the tree's file. The plugin registers the
same four events, so a second copy of them runs klin twice on every event: two
gates race for one turn stamp, and the prompt counter of 6.2 moves by two.
Each host's adapter knows where that host lists its enabled plugins. Claude
Code lists them under `enabledPlugins` in its settings files: for a write into
a tree klin reads the tree's, the local ones beside them and the user's, and
for a write into the home directory the user's alone, because a plugin one
repository enables gates that repository and not the machine. Codex CLI lists
them as `[plugins."klin@<marketplace>"]` tables in `config.toml`, on unless
the table says `enabled = false`, and klin reads the tree's and the user's the
same way. A write into a tree is refused the same way by a user file that
holds klin's entries.

klin replaces a host's settings file whole, through a neighbour and a rename,
so a run that dies partway leaves the file it found. It follows a path that is
a link, so a settings file kept in a dotfiles tree stays a link, and it keeps
the permissions the file had.

`--global` moves both the detection and the write to the host's user-level
directory: `~/.claude/settings.json`, `~/.cursor/hooks.json`,
`~/.codex/hooks.json`. Everything else is the same, so a host with no adapter
is refused under `--global` with the message the per-tree form gives, and
gains `--global` when its adapter lands. A global install is not committed,
so the write says it covers every repository rather than asking for a commit,
and a person who chooses it accepts that an agent can remove the lines.

### 19.4 CI

The Action installs the pinned version and runs `klin gate --strict` with
`fetch-depth: 0`, and an `args` input appends flags such as `--gate` names or
`--sarif`. The version comes from the `version` input, then the `version` key
in `klin.json`, then the tag the workflow pinned the Action at, then the
latest release. The Action runs the install script under that release's tag
URL, and that script verifies the checksum, so a mismatch fails the job
before any gate runs. A workflow without the Action runs the same script and
the same command.

### 19.5 Upgrades

A new klin version may change a measurement. Under the base commit model both
trees are measured by one binary, so an upgrade changes nothing about any
verdict except where a new check applies. A new derivable check runs on the
first stop after the upgrade, against a derived ceiling from the base tree,
so it is green on arrival. The plugin pins its own version and upgrades when
the plugin does, through `/plugin marketplace update` or the host's
auto-update. Every other route upgrades when the person asks. `klin update`
runs the `klin-update` beside the binary, or the one PATH resolves, which
installs the newest release over the current one, and its exit code is the
updater's. Where no updater is found, `klin update` says so, names the
installer, and exits 2. ADR 0029 records that one tag names every route.
