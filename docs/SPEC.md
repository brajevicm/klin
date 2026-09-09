# klin Specification

Status: Draft v0, 2026-09-08

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
| 0007 | Config keys match cleat's. | One key vocabulary across sections. The differential test goes. | ADR 0009 already weakened the reason to a preference. `sources` in one section and `roots` in the next is a cost a user pays for a test klin no longer needs. |
| 0003 | A file no grammar reads is exit 2 everywhere. | In the hook it is a NOTE. Outside the hook it stays exit 2. | An agent cannot fix a grammar. Blocking a stop on it costs a turn per stop with no remedy. |
| 0011 | A non-reader naming a guarded path is refused. | It is `ask`. Only a clear write is `deny`. | Four read-only commands were refused in the session that wrote this draft. Section 9.4. |
| 0004 | The host's cap on consecutive blocks bounds the loop. | klin bounds its own blocks. | The cap is not in the host's current documentation. Section 9.3. |
| 0015 | klin's state lives in `.klin/` at the tree root, with one `.gitignore` line. | State lives in the git directory, or under `KLIN_STATE_DIR`. No ignore line. | Per-worktree state has a native home that git never tracks and never lists. Section 7.4. |
| 0002 | The plugin ships no binary. | The plugin ships a `bin/klin` wrapper that fetches the pinned release on first run. | The first reason, a pin beside committed baselines, went with ADR 0009. The second, `bin/` unavailable for organization-distributed plugins, is handled by falling back to PATH. Section 19. |
| 0014, one line | The turn stamp is not guarded, because a report leaves nothing to gain. | The stamp is in the guarded set, and a missing stamp widens the window instead of closing it. | The stamp now holds the verdict that keeps a window open. A stamp that is gone is restored from its ref, or the stop judges the whole branch, so deleting it buys nothing. Sections 6.2 and 9.4. The rest of ADR 0014 stands. |

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
files to the guard's `ask` list (9.4), the empty test body to `stubs` (8.2),
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
- Executing a project's test suite, or running anything five times.
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

A function moved between files with its body unchanged SHOULD match its old
site. The RECOMMENDED second pass matches unmatched findings to unmatched
entries by a whitespace-normalized body hash, across files. This is additive
to ADR 0008 and does not change the primary key.

### 4.5 Finding

One site with the values a check measured there. Fields:

- `file` (string) repository-relative path, REQUIRED
- `line` (integer) 1-based, REQUIRED where a line exists
- `text` (string) the declaration line, REQUIRED, part of the identity
- `values` (object) metric name to number or string, REQUIRED
- `outcome` (`new` | `worsened` | `held`) set by the engine, never by a check.
  A note about a site, such as an unmatched accepted entry or an unparsed
  file, is not a finding. It is a separate record under `notes` in the JSON
  (11.2) and carries no `outcome`.

A value the check ratchets on is one where higher is worse. A check MUST name
those values. A value that is not ratcheted is carried for the report only.

### 4.6 Check and Gate

A check is one measurement and the judgement behind it. A gate is one
configured or derived instance of a check. A check declares:

- `name`, the command name and the `--gate` name
- `section`, the config key it reads
- `compares_to_base`, whether it matches sites between two trees. Every check
  with a `derive` MUST compare to the base, so that a tree with no config is
  green (5.1). A check that judges one tree against a number exists only for
  a number a person pinned.
- `takes_scope`, whether a changed-file list narrows it
- `derive`, a function from a surveyed tree to a section, or none when the
  check cannot apply without a person, such as `sarif`
- `cost`, an ordinal that orders the run cheapest first

A check with no `derive` runs only when its section is present. Every other
check runs unless its section is `false`.

### 4.7 Ceiling

The value a measure may reach before its gate fails. A ceiling is derived, a
pinned number, or a pinned dated schedule. Section 7.3.

### 4.8 Accepted entry

Debt a person allows, keyed like a site, with the value they allow. Only a
person writes one, in a reviewed commit (ADR 0009). An entry that matches
nothing is a NOTE, and a failure under `--strict`.

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
- `radius` (object) OPTIONAL, ADR 0014. Derived from history when absent.
- one key per gate, named for its section, or `false` to exclude the gate

A key klin does not know MUST be an error naming the key. A section with a
`baseline` key MUST be an error saying the key is gone (ADR 0009). A section
MAY pin some keys and leave others to derivation. A pinned key MUST print as
`pinned` beside the derived ones.

### 5.3 One key vocabulary

Every section uses the same names for the same things: `roots`, `languages`,
`exclude`, `skip_dirs`, `ceilings`. `sources` in the complexity section is
renamed to `roots`. A run that meets the old name MUST say what the new name
is. cleat's keys are no longer a constraint, and the differential test that
needed them is retired.

### 5.4 Derivation rules

Each check documents its rule. The rules for the shipped checks:

- `roots`: directories under the tree root that hold source files of a known
  language, excluding the default skip set, merged up to the shallowest
  directory that holds nothing but source. The set is the union of the
  derivation commit's survey and the `after` walk (4.3). Test roots are the
  subset whose name or files match the language's test convention.
- `languages`: the languages of the files under `roots`, in the derivation
  commit and in `after`.
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
host `--host` names. Section 19.3. Like `init` itself and `turn reset`, it
writes a guarded target, and it is a person's command. The guard refuses it
from an agent.

`init` is a convenience, not a step. A tree with no `klin.json` is fully
gated.

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
stop, and a prompt counter. `klin gate --hook` writes the verdict. `klin
radius` applies the rule above and raises the counter by one on every
session start and prompt submitted, whether or not the stamp moved. The
counter is what makes "once per turn" in 9.3 literal, because the stamp
itself moves only after a green stop. The stamp is guarded (9.4).

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
in the state directory holds the time, the verdict and the prompt counter
beside the commit id.
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
finding matches at most one entry. Identical sites in one file match by line
order, then by closest value.

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

klin's own state is three things: the turn stamp, the build stamp, and the
survey cache. All are per working tree. The cache is safe to delete. The two
stamps are in the guarded set (9.4), because each holds a fact that keeps a
block alive. Deleting the turn stamp buys nothing, because a stop without one
judges the whole branch (6.2). They live in the state directory:

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
no longer exists. It MUST NOT remove a stamp. A repository path that does not
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
| `inventory` over tests | deleted test file, deleted test function | test file path, or test function site | `missing` rises | yes | #45, #69 |
| `lockfile` | dependency added without a lockfile entry, pin removed | manifest + name | new against `before` | yes | #58 |
| `sarif`, `after` only | anything a linter reports, on a line the window changed | file + rule + message | new on a changed line | no | #47, section 8.3 |

`inventory` has two identities. A test file is keyed by path. A test function
is keyed like a complexity site, file plus declaration line, and only
functions the language's test convention marks count, such as a `#[test]`
item, a `test_` function or an `it(` call. It ratchets one value, `missing`,
which is 0 for every test site in `before` and 1 for a site `after` no longer
holds, so a vanished site is `worsened` under the one judge of 16.4. The
remedy names the vanished site and says to restore the test, or to record in
the accepted list why it went, and a person writes that entry. The second
pass of 4.4 matches a renamed test by body hash before it counts as missing,
so a rename with the body unchanged is `held`. A rename that also edits the
body is `worsened`. A deletion that removed the subject too is a NOTE: for a
file, the subject file went in the same window, and for a function, the file
that held it went. The subject of a test file is the file in `before` whose
path equals the test's path with the test affixes stripped: a `test_` or
`spec_` prefix, a `_test`, `_spec`, `.test` or `.spec` suffix before the
extension, a `Test` or `Tests` suffix on the basename as in `FooTest.java`
or `FooTests.swift`, and a `tests/`, `test/`, `spec/` or `__tests__/`
directory segment. The table is fixed in the binary and printed with the NOTE.
Deleting a test that fails is the cheapest route to green in section 1, and
the file-level inventory alone does not close it.


`lockfile` proves one thing: every dependency the manifest names has an entry
in the lockfile beside it, and no pin the base held is gone. It cannot prove
that a package exists in a registry, because it runs offline. A dependency
that does not exist fails the project's own install, which the `build` step
runs. Workspace members, path dependencies and optional dependencies are
implementation-defined and MUST be documented per manifest format.

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
only in the table. #106 ships the line patterns first and the body shapes,
which need the function walk, in a second ticket.

The escapes table gains three rows for test-disabling constructs it lacks:
`fit(`, `fdescribe(` and `pytest.mark.xfail`. `skipif` is not a row, because a conditional skip states which platforms a test supports. The
other focus and skip markers, `.only`, `.skip`, `xit`, `#[ignore]`,
`@Disabled`, `t.Skip` and `XCTSkip`, are already there.

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

Each result is keyed by file, rule id and message. A result fails when its
line falls inside a hunk the window changed. A result on a line the window did
not change is held, whatever the base held there. With `differential: true`
the tool already reports only what is new, so every result fails. Scoping to
changed lines, not changed files, is what keeps an agent that touches a file
with thirty old warnings green. A reformat that moves every line is the known
weakness, and the radius report already names such a turn.

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

The reference extractor resolves a reference by name to every declaration of
that name under the roots. That errs toward "referenced", so `dead-symbols`
and `reachability` fail less, never more. Rust, TypeScript and Python come
first, and a file in a language the extractor has no table for is counted as
not measured on the coverage line of 8.6, so a green run over such a tree is
visibly a run over nothing.

A `test-hygiene` check, a count of habits across the test roots against a
dated ceiling, was considered and is not a check. A habit that rose is an
escapes `patterns` row with `roots` set to the test roots, and a schedule on
a whole-tree total fails a tree nobody changed, which is the forced paydown
7.3 refuses.

### 8.5 Tier 3: defer with a reason

`layering` (#50): the compiler's module graph is the check, per cleat's own
strategy. `guard-suites` (#43) and `manifests` (#44): one stack each.
`db-migration-safety` (#57): deterministic for raw SQL only. `asset-path`
(#59): the ticket expects false positives, which fails criterion 1 in
spirit. `flaky-test-runner` and an MCP server: refused in ADR 0008.

### 8.6 Per-check contract

Every check MUST:

- state its identity rule, its ratcheted values and its derivation rule in
  its module docstring
- print one remedy per failure that names what to change
- print, per failure, the site it matched in `before` or the accepted entry it
  matched, and the value on each side, so an agent fixes the right thing and
  a person can dispute a wrong match. A `new` finding says that nothing
  matched.
- print one `OK:` line with what it judged on success, plus any `NOTE:`
  lines, and nothing else. What it judged includes the coverage: how many
  files it found, measured, excluded and could not read, so a green run over
  an unexpectedly small scope is visible on its one line.
- name every file that is present in both trees, was measured in `before`,
  and was not measured in `after`. The union of roots in 5.4 means a check
  can only discover more, so such a file left through an exclusion, a file
  the grammar stopped reading, or a discovery rule the tree no longer meets.
  In the hook it is a NOTE. Under `--strict` it is exit 2, per section 10.
- name a file it could not measure. Outside the hook that is exit 2, with or
  without `--strict` (ADR 0021). In the hook it is a NOTE, because the agent
  has no remedy.
- run under `klin gate` and under its own subcommand with the same output
- carry tests through the binary only, on a throwaway tree with a base

## 9. Hook Protocol

### 9.1 Host adapter

One `Host` value, chosen from the event's shape or from `--host`, maps a host
event to one internal record:

- `tool` (string), `file_path` (string), `command` (string) for the guard
- `blocked_before` (bool), the host's flag that the hook already blocked this
  turn
- `session` (string) when the host sends one

And maps one internal decision to the host's output. For Claude Code:
pre-tool decisions go out as `hookSpecificOutput.permissionDecision` with
`allow`, `deny` or `ask` and a reason. Stop blocks are exit 2 with the report
on stderr. Prompt and session-start text go to stdout on exit 0.

In hook mode the exit code is the host's protocol, not the verdict. Exit 2
means "block this stop", whatever caused it. The verdict of section 4.9 lives
in the report and in the `turn` file. Outside hook mode the exit code is the
verdict.

Cursor and Codex CLI adapters are separate tickets (#67, #68). Until they
exist the README MUST NOT name those hosts. The adapter is the only module
that reads a host's JSON.

### 9.2 Events

| Event | Command | Blocks | Writes |
|---|---|---|---|
| session start | `klin radius` | never | `turn` per 6.2, and its prompt counter |
| pre-tool | `klin guard` | deny or ask | nothing |
| prompt submitted | `klin radius` | never | `turn` per 6.2, and its prompt counter |
| stop | `klin gate --hook --changed` | each stop while the build fails, up to eight per turn, and once per turn for gates | `build-blocked`, and the verdict in `turn` |

The hook lines are the same on every host and call `klin` from PATH:

```
klin guard
klin radius
klin gate --hook --changed
```

The host adapter reads which host called from the event, so no flag is
needed in the hook line. `--host NAME` overrides detection.

### 9.3 The block-once policy

ADR 0004 and ADR 0012 hold in policy. A build failure blocks each stop until
the tree builds. A gate failure blocks the first stop and reports on the
second. The build stamp in the state directory carries the fact between the
two processes.

ADR 0004 relies on the host's cap on consecutive blocks. That cap is not in
the current Claude Code documentation. klin MUST bound its own blocks (ADR
0022). A gate failure blocks once per turn. A build failure blocks at each
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

- `deny`: an edit tool whose `file_path` is `klin.json`, a host's hook file
  or CODEOWNERS, a redirect onto one of those, a whole-tree restore, `init`
  in any form, and `turn reset`. The reason names the file and says a person
  changes it in a reviewed commit, or names the command a person runs
  instead.

The guarded set is: `klin.json`, each host's hook file, CODEOWNERS, the state
directory of 7.4, and `refs/worktree/klin`. The state directory and the ref
are in the set for a different reason from the others. Nothing in them needs
a reviewed commit to restore, and a deleted stamp is restored from the ref
or replaced by a branch window (6.2), so a deletion gains nothing. They are
in the set so that a write to them is a question a person answers, not a
silent act. No `deny` is needed for them.
- `ask`: a shell command outside the reader list whose arguments name a
  guarded path, and an edit tool or a redirect whose target is the state
  directory or the ref. The reason quotes the token that matched. The person
  decides.
  The reader list of ADR 0011 gains `git rev-parse`, `git cat-file`, `git
  for-each-ref`, `du`, and `find` without `-delete`, `-exec`, `-execdir` or
  `-ok`, so an agent can read a stamp or list the state directory without a
  question.
  Also an edit tool or non-reader command that names a verification file. The
  verification files are a fixed table in the binary, not a config key: lint
  configuration such as `.eslintrc*` and `eslint.config.*`, test
  configuration such as `pytest.ini`, `jest.config.*` and the `[tool.pytest]`
  and `[tool.coverage]` tables' files, coverage thresholds such as
  `.coveragerc` and `codecov.yml`, and CI workflows under `.github/workflows`.
  An agent that edits one of these can weaken every check that reads it
  without touching `klin.json`, and klin cannot tell a loosening from a fix,
  so a person looks. The table is `ask`, never `deny`, and the reason names
  the file and says why klin asked.
- `allow`: everything else, including any reader naming a guarded path, and
  any glob that does not match a guarded name.

A glob matches a guarded name only when the glob, read as a pattern, matches
it. An empty prefix MUST NOT match. Splitting a command into segments MUST
honor single and double quotes (issue #90). The body of a heredoc is data:
the guard matches the command words and every redirect target, including a
command after the heredoc's terminator, and does not match the text between
the delimiter and the terminator.

The guard MUST NOT read the configuration. It runs before the config loads.
It MAY read `KLIN_STATE_DIR` and run `git rev-parse --git-dir` to learn the
state directory. It MUST finish in under 50 milliseconds, because it runs on
every tool call.

### 9.5 What the hook prints

On a block, one lead line that says how many gates failed and what to do,
then each failing gate's own output, then the derived values the run used.
Never the command that accepts debt, and never `turn reset`. On a second stop,
the same report and a line saying the window stays open until a person fixes,
accepts or resets it.
The `--json` form is available for a host that reads JSON.

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
rows. `OK:` for a passing gate's one line. `FAIL:` for a failure with its
remedy under it. `NOTE:` for a note. One `window:` line first. One
`derived:` line per derived value, after the rows.

### 11.2 JSON

One object on stdout. Fields:

- `window` `{kind, before, after, how}`
- `derived` list of `{section, key, value, rule}`
- `gates` list of `{name, status, findings, notes, coverage}`, where
  `coverage` is `{found, measured, excluded, unreadable}` file counts
- `findings` entries per 4.5 with `id`, `condition`, `fix_advice`,
  `ceiling`, and `matched`, which is the `before` site or accepted entry with
  its values, or null for a `new` finding. `id` is a hash of the gate name,
  the file and the declaration text, so it is the site identity of 4.4 in one
  token, and a harness can follow one finding across stops without parsing
  the rest. The id follows the path, so a file rename changes it while the
  site of 4.4 survives.
- `exit` integer

A finding has no column, so the JSON carries none rather than a wrong one.

### 11.3 SARIF

`--sarif PATH` writes SARIF 2.1.0 to the file `PATH` names, with one rule per
gate and one result per failing finding (#65). Text output on stdout is
unchanged by the flag, which is why the log goes to a file and not to stdout
the way `--json` does. `--sarif` with `--json` is a usage error.

## 12. Determinism

- Every `git diff` klin runs MUST pin `--diff-algorithm=histogram` and pass
  `-M` or `--no-renames` by name (ADR 0014).
- Findings MUST be sorted by file then line before matching and before
  printing.
- Grammars are compiled into the binary. A grammar version change is a klin
  version change, and the survey cache key includes the version.
- No check MAY read the network.
- The only clock is a pinned dated ceiling (5.5), read in UTC, and
  `KLIN_TODAY` overrides it. The report age check of 8.3 compares file
  times and is the one other place time enters.
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
| A file no grammar reads | Outside the hook: the gate names it and exits 2, other findings still print. Hook: a NOTE. |
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
and refuses its edits to the config and asks about its writes to the stamp.
Nothing prevents a PATH
shim or a `chmod -x`, and the guard sees only the tool calls the host shows
it. This level is what a person gets with no CI, and this document makes no
stronger claim for it.

### 15.2 Enforced level

Feedback level plus: a CI run with `--strict`, on a checkout the agent never
touched, against a protected branch, with `klin.json`, the workflow, the hook
settings and CODEOWNERS under CODEOWNERS. At this level a gate holds against
an agent, and loosening it takes a reviewed commit by a person.

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
  if stamp is None and not exists(state): move_stamp()       # first session here
  elif stamp is not None and stamp.last_verdict == GREEN: move_stamp()
  elif stamp is None: note("stamp deleted, the next stop judges the branch")
  if exists(state/turn): bump_prompt(state/turn)   # prompt += 1, whether or not the stamp moved
  report_radius(stamp)                             # a deleted stamp gets no file here

turn_reset():                          # a person's command, denied by the guard
  move_stamp(); print("a person moved the window")

move_stamp():
  tree   = write_tree(index=state/index, add_all=True)
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/turn", commit)
  write_atomic(state/turn, commit=commit, parent=HEAD, time=now, last_verdict=None,
               prompt=current_prompt(state/turn))

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
  (failed, errored) = run_gates(config_or(survey), window, scope=changed)
  write_verdict_atomic(state/turn, GREEN if failed == 0 and errored == 0 else RED)
  if failed == 0 and errored == 0: return 0
  if count.gate_spent: report(); return 0
  if count.builds == 0 and host.blocked_before(event): report(); return 0
  count.gate_spent = True; write_atomic(state/build-blocked, count)
  block(report)
```

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
  before = check.measure(window.before, section) if check.compares_to_base else []
  after  = check.measure(window.after,  section, before)   # only inventory reads `before`
  entries = before + accepted(config, check.name)
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
A site the accepted list names is matched like any other, so a person accepts
a deletion the same way they accept any other debt. A vanished site whose
subject went in the same window is a NOTE, not a finding.

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
  function keeps its site within a file and across files, a copy of a
  function beside its original is `new` and does not inherit the original's
  match, a lowered ceiling fails no held site, accepted entry holds a site,
  unmatched accepted entry is a NOTE and a strict failure, a failure prints
  the site it matched and both values.
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
  deleted test function fails, a renamed test function with its body
  unchanged is held, a deleted test function whose file went too is a NOTE.
- `sarif`: a result on a changed line fails, on an unchanged line in a changed
  file is held, `differential` fails every result, `run` writes the report
  before it is read, a report older than a changed file is ERR.
- `lockfile`: a manifest entry with no lockfile entry fails, a removed pin
  fails, a path dependency is not judged.
- Coverage: a file present in both trees and measured in `before` only is a
  NOTE in the hook and exit 2 under `--strict`, whether it left through an
  exclusion or a grammar error.
- Concurrency and worktrees: an old green stop that finishes after a new red
  stop does not write green, two worktrees keep independent stamps through
  `git gc --prune=now`, a stop that cannot take the lock writes no verdict.
- Guard, verification files: an edit to a lint config, a test config, a
  coverage threshold and a CI workflow is `ask`, and a read of each is
  allowed.

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
  ask route including a write that names the state directory or
  `refs/worktree/klin`, every reader allowed including `git rev-parse` and
  `git cat-file` on the ref and `find` over the state directory, `find
  -delete` over it is `ask`, glob does not match by empty prefix, quoted pipe
  does not split, under 50 milliseconds.
- Init: pins exactly what the run would derive, writes only the config,
  `--add` leaves `false` alone, `--force` re-pins, never touches
  `.gitignore`, `--hooks` writes each host's file and leaves an existing
  entry alone.
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
- [ ] The guard denies `init` and `turn reset`, asks on the state directory,
      `refs/worktree/klin` and the verification files of 9.4, and allows the
      plumbing readers

- [ ] Survey at run time from the derivation commit, cached by it, derived
      values printed. Derived numbers come from the derivation commit's own
      paths. Roots, languages, documents and manifests are the union of that
      survey and the `after` walk, and a site under a path the survey did not
      hold is `new`.
- [ ] `doc-citations` and every other derivable check compare to `before`
- [ ] No source root is exit 2 under `--strict`
- [ ] Derived complexity ceilings, floor and minimum sample, floor for a new
      language
- [ ] Pinned dated ceilings in every check that takes a ceiling, in UTC, date
      printed
- [x] One key vocabulary, old names print the new one, differential test
      retired
- [ ] Unreadable file is a NOTE in the hook
- [ ] Coverage counts on every `OK:` line, matched site and both values on
      every failure, a finding `id`, all in the JSON, and the coverage
      regression NOTE and strict failure
- [ ] Host adapter, Claude Code first, README stops naming other hosts
- [ ] Three escapes rows: `fit(`, `fdescribe(`, `xfail`
- [ ] Cross-file move matching by body hash (4.4)
- [ ] `inventory` over test files and test functions, with the
      deleted-subject NOTE and the body-hash rename match
- [ ] `stubs`, sharing the escapes engine, executable function bodies only,
      the empty test body included, with a legitimate-change fixture per row
- [ ] `sarif`, `after` only, delete `report`, `run`, read `report`, scoped to
      changed lines
- [ ] `lockfile`
- [ ] `CONTEXT.md` takes Window and Derived, README names the two
      conformance levels
- [ ] New ADRs for each row of section 0 that is accepted, and one for the
      stamp as a guarded, parented, ref-held commit

Next, after core is green, each with an unsolved problem named in section 8:

- [ ] `duplication`
- [ ] `sarif` with `compare`, once the dependency problem of 8.3 has a design

Distribution, in this order, because each step depends on the one before:

- [ ] Release pipeline: four binaries and a checksum file per tag (#62)
- [ ] Install script with `--version`
- [ ] The Claude Code plugin with `hooks.json` and the `bin/klin` wrapper (#66)
- [ ] `init --hooks` for Cursor and Codex, and their host adapters (#67, #68)
- [ ] The GitHub Action
- [ ] Homebrew tap, `cargo install`, npm wrapper (#64)

Recommended:

- [ ] Configuration reference (#18) generated from each check's declared keys
      and derivation rules
- [ ] `--sarif` output

Before calling it 1.0:

- [ ] Performance numbers from section 13 recorded on a fixture
- [ ] A task comparison with and without klin on a small set of agent tasks,
      recording regressions caught, legitimate changes blocked, extra repair
      turns and hook latency. This is a benchmark, not a test, and it is what
      shows the tool is useful rather than correct.
- [ ] The hook-output facts in 9.3 verified against the host's documentation
- [ ] Cursor and Codex adapters, or the README stays silent on them

## 19. Installation and Distribution

Two things are installed, and they stay separate. The binary is one static
file per platform. The hooks are three lines of host configuration that call
it. Every route installs the binary once and then writes hook lines.

### 19.1 The binary

A pushed tag builds the binary for macOS and Linux, on x86_64 and arm64, and
attaches the four binaries and one checksum file to a GitHub release. Every
route below downloads from that release and MUST verify the checksum. The
routes, in order of least friction for the person:

1. An install script: `curl -fsSL <url>/install.sh | sh`, with `--version` to
   pin one release. Puts `klin` on PATH.
2. A Homebrew tap, for macOS and Linux users who already have brew.
3. `npm install --save-dev klin`, for a JavaScript project. The package holds
   no compiled code. Its install step downloads the release for the host and
   verifies it. The package version equals the release tag, so the lockfile
   pins the binary.
4. `cargo install klin`, for a Rust user. Free once the crate is published.
5. A GitHub Action, `klin-action`, that installs a pinned version and runs
   `klin gate --strict`. For CI only.

Cross-compilation for Windows is not a target of this draft. The Codex hook
system is not available on Windows either.

### 19.2 Claude Code: the plugin is the whole install

The plugin holds `hooks.json` with the three hooks of 9.2, one skill that
tells the agent how to read a failure and what it may not touch, two slash
commands that run the gates and list them, and a `bin/klin` wrapper.

The wrapper is a shell script. On first run it downloads the release the
plugin version pins into `~/.cache/klin/bin/<version>/klin`, verifies the
checksum, and executes it. Every later run executes the cached binary with no
network call. When the download fails, the wrapper prints one line saying so
and exits 0, so a turn is never blocked by a missing network. This is the one
place klin touches the network, and it is install, not measurement.

Plugins add `bin/` to the Bash tool's PATH. Where `bin/` is unavailable,
which is the case for plugins distributed through organization settings, the
hooks find `klin` on PATH from a route in 19.1, and the skill says which
command installs it. When neither is present the Stop hook says so once and
lets the turn end.

With the optional config of section 5, installing the plugin is the complete
install. No `init` runs. The first stop is gated.

This reverses ADR 0002. Its first reason, a version pin beside committed
baselines, went with ADR 0009. Its second reason is handled by the PATH
fallback above. A version difference between the wrapper's binary and a CI
binary is a NOTE per 5.2, not a failure.

### 19.3 Cursor and Codex: a hooks file in the repository

Neither host has a plugin that can carry a binary, so the binary comes from
19.1 and `klin init --hooks` writes the host file:

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
hooks, and CODEOWNERS SHOULD cover them. This command edits guarded files, so
the guard refuses it from an agent, like `init` itself.

### 19.4 CI

The Action installs the pinned version and runs `klin gate --strict` with
`fetch-depth: 0`. A workflow without the Action runs the install script with
`--version` and the same command. The `version` key in `klin.json`, when
pinned, is the version the Action installs by default.

### 19.5 Upgrades

A new klin version may change a measurement. Under the base commit model both
trees are measured by one binary, so an upgrade changes nothing about any
verdict except where a new check applies. A new derivable check runs on the
first stop after the upgrade, against a derived ceiling from the base tree,
so it is green on arrival. The plugin pins its own version and upgrades when
the plugin does. The script and the package managers upgrade when the person
asks.
