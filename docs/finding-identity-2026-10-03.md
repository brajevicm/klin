# Versioned structural identity for ratcheted findings, 2026-10-03

This note belongs to #425, under #358. It asks whether a ratcheted finding
should carry a versioned structural identity, so that an ordinary edit to one
underlying site stays the same debt, and so that an ambiguous or duplicate
site is handled conservatively instead of guessed.

It is research only. It changes no shipped ratchet behavior, no public JSON,
no accepted-entry contract and no SPEC semantics. A positive result does not
authorize an implementation.

The baseline is `ac861ec2`, the commit this work starts on.

## Terms

These words belong to this note. They are not terms of `CONTEXT.md`.

- A **key** is a family's structural name for one site in one file, such as
  `impl Config > new` or `fetchRoomRatePlans > keyFor`. It holds no metric
  and no line number.
- An **identity** is the envelope around a key: the family and version, and
  the state `identified` with a key, or `ambiguous` with a reason.
- A key **persists** when the other tree holds a site with the same key in
  the same file. The census for that question is every site of the family in
  that file, and not only the sites over a ceiling.
- The **prototype** is a patch to klin at `ac861ec2`,
  [`prototype.patch`](finding-identity-2026-10-03/prototype.patch). It is not
  klin code. It carries the identity as a string in a finding's `values`,
  which a real implementation must not do (section 9). The note calls its
  builds `proto3`, `proto4` and `proto5`. `proto4` adds one fast path to
  `proto3` (section 8). `proto5` is the checked-in patch: it compares only
  known versions, derives keys only for Rust, TypeScript, TSX and Python, and
  sends files of other languages through today's first pass (section 9). For
  its version cases, two prototype-only variables,
  `KLIN_PROBE_BEFORE_VERSION` and `KLIN_PROBE_AFTER_VERSION`, set the
  version of each tree's identities, or `none`. All three builds give the
  same result on the 43 cases that existed before `proto5`.
- An **exposure replay** runs `complexity` with the ceiling `cc 0`, so every
  function is a site and every edit reaches the matcher. A **floor replay**
  runs the floors of the derived ceilings, `cc 10` and `lines 25` (SPEC 5.4).
- **TS-A** and **TS-B** are two private TypeScript repositories of the owner.
  The note publishes their counts and no file, line or code of theirs.

## Method

1. The edit matrix,
   [`probe.py`](finding-identity-2026-10-03/probe.py), builds 61 throwaway
   trees, a base on `main` and an edit on `work`. It runs
   `klin gate --gate <gate> --json` from each binary, once whole with
   `--strict` and once with `--changed`. It also records the prototype's key
   for every site of both trees, which it reads by measuring each tree
   against an empty base. The result is
   [`matrix.md`](finding-identity-2026-10-03/matrix.md).
2. The census and replay,
   [`replay.py`](finding-identity-2026-10-03/replay.py), clones a repository.
   The census measures every function of `HEAD` as a site and counts the
   identity states. The replay takes the last 200 commits that are not
   merges, sets `main` to each commit's parent, and runs `--changed` from the
   baseline and the prototype. A difference is a site whose outcome the two
   binaries disagree on.
3. Three repositories: klin (Rust, with TypeScript under `benchmark/`), TS-A
   and TS-B. The klin data is in
   [`exposure/`](finding-identity-2026-10-03/exposure) and
   [`floor/`](finding-identity-2026-10-03/floor).
4. Every difference was classified by machine: is the finding's declaration
   text absent from the parent's file? Every difference where the prototype
   is stricter than today was then read by hand.
5. Performance was measured with `hyperfine` on a klin clone with 20 changed
   files, and with the `structural_300k` and `structural_1m` `warm20` rows
   under `KLIN_BIN` (section 8).

## 1. Current matching failure modes

Today a site is the file plus the trimmed text of the declaration line (SPEC
4.4, ADR 0008). The judge groups by `(file, text)`, pairs one to one by the
rank of 16.5, and then gives unmatched sites a body-hash pass across files.
The matrix and the replays show three failure modes.

### F1. A declaration-line edit makes held debt `new`

A change to the declaration line changes the text, so the primary match
fails. The body-hash pass saves the site only when the body did not change.
A parameter rename changes the body too, because the body uses the parameter.

Fixtures: `signature-and-body`, `visibility-added`, `parameter-rename`,
`parameter-rename-ts`, `parameter-rename-tsx`, `nested-signature`,
`bound-arrow-ts`, `static-and-instance-ts` and `getter-setter-ts`. Today each
one reports a function that existed at the base as `new`.

Real code, at the floor ceilings: 10 such sites in klin, 2 in TS-A and 33 in
TS-B, over 200 commits each. In one TS-B commit, the `complexity` gate failed
only on such a site, so the prototype changes that gate's verdict to a pass.
The replay ran only that gate, so it does not show whether another gate also
failed that commit. In the exposure
replay the count is 94 in klin, 13 in TS-A and 97 in TS-B.

A false `new` is a wrong claim about the agent's work. It also hides a real
`worsened` site: of the 45 floor sites, 34 grew, and today klin calls them new
instead of naming the value they grew from.

### F2. Same-text twins under different owners pair across owners

Two sites with the same declaration text in one file share one group. The
rank prefers equal values over line distance (16.5), so when one twin
improves and the other worsens, each can take the other's entry.

Fixtures:

- `two-owners-swap-rust`, `two-traits-swap-rust`, `two-owners-swap-python`,
  `two-owners-swap-ts`: `A::run` goes from cc 5 to 3 and `B::run` from 3
  to 5. Today the gate passes. `B::run` got worse and nothing says so.
- `nested-same-name-two-parents`: the same miss for a nested `inner` under
  two parent functions.
- `twin-crosses-ceiling`: `B::run` grows past the ceiling while `A::run`
  drops under it. Today `B::run` is held by `A::run`'s entry.
- `two-owners-misnamed-rust`: the gate fails, but names `A::run`, which
  improved, as worsened.

Real code: in TS-B, three `execute: async ({ path: relPath }) => {`
callbacks sit in one file under three factory functions. One of them grew to
cc 2 and 11 lines, and today it reads as `held`. Nine more rows are naming
corrections. The count of new sites is the same, but today klin names the
wrong one:

- klin `ef451b2c`: `Pattern::key` became `Pattern::record`, and the free
  `fn record(` gained a lifetime. Today the renamed method takes the free
  function's entry.
- TS-A: a commit added `fetchBatchSingleFlight` with its own `keyFor` above
  an existing `fetchRoomRatePlans > keyFor`. Today klin calls the old one new.
- TS-B: three commits added routes to a generated route tree, where every
  route holds `getParentRoute: () => ...` with the same text. Today line
  distance picks which route is new. Seven times it picks an existing one.

Only the exposure replay saw F2. No F2 case reached a site over the floor
ceilings in these 600 commits.

### F3. A same-text twin blocks a move across files

klin `c5d90027` moved `Tree` from `src/project.rs` to `src/tree.rs`. Before
the commit, `project.rs` held two `pub fn root(&self) -> &Path {`, one for
`Project` and one for `Tree`. After it, `project.rs` holds one. Today the one
that stays can take either entry, so the moved method may find no lost entry
for the body-hash pass, and today it is `new`. One exposure row.

The fixture `twin-moved-to-other-file` shows it: `A::run` and `B::run` share
their text, `B`'s impl moves to `b.rs`, and `A::run` lands on `B::run`'s old
line. Today `A::run` takes `B::run`'s entry, and the moved `B::run` is `new`.
With identity both are held.

### What works today

`comment-above`, `sibling-insert`, `sibling-reorder`, `body-edit`,
`signature-only`, `rename-same-body`, `file-move`, `same-name-new-body`,
`delete-add-replacement` and both `added-duplicate-occurrence` cases give the
desired outcome today. The identity must not change them, and it does not.

The whole run and the `--changed` run gave the same outcome on all 61 cases.
The one cell that differs is `accepted-and-base-param-rename`, where only the
whole run passes `--strict`.

## 2. Candidate identity envelope

The shared contract is the envelope only:

```text
identity:
  version   family and rule version, such as "complexity/1"
  state     identified | ambiguous
  key       when identified: the family's key
  reason    when ambiguous: anonymous | computed | ancestry | duplicate
  persists  when identified: whether the other tree's file holds the key
```

The family owns the key, and the key never holds a measured value, a line
number or a body hash (ADR 0008 refuses a metric in the key).

The file stays outside the key and inside the group, as today. A key is
unique within one file at most, and the judge compares keys only within one
file. The cross-file pass stays the body-hash pass.

`persists` is a fact of the two trees, so the gate computes it, because only
the gate measured both trees. The matcher cannot compute it from the findings
alone: a site under the ceiling is no finding, and it still persists
(`twin-crosses-ceiling`).

## 3. Per-family identity rules

### complexity: the proving family

The key is the named path from the file to the function, outermost first,
joined by ` > `. The prototype reads it while the existing walk is at the
function node: it climbs the node's ancestors and reads names the grammar
already gives.

- A node with a `name` field gives its name. This covers Rust `mod`, `trait`
  and `fn`, TypeScript `class`, `namespace`, functions and methods,
  `const f = ...`, and Python `class` and `def`.
- A Rust `impl` gives `impl Type`, or `impl Type as Trait` for a trait
  implementation. The type loses its generic arguments, so `impl F<u8>` and
  `impl F<u16>` are the same owner, and two `fn new` under them are
  duplicates (section 4).
- A TypeScript method gives `static`, `get` or `set` before its name, so a
  static member, an instance member, a getter and a setter of one name are
  four keys.
- A function with no name of its own takes the name of the binding the
  syntax gives it: `const f = () => ...`, `key: () => ...`, a class field,
  or an assignment. With none, the function is anonymous.

Example keys: `impl Derivation > walk`, `impl Pattern > record`,
`fetchBatchSingleFlight > keyFor`, `A > static make`.

The rule reads grammar fields, so what it derives depends on the grammar.
`complexity/1` therefore covers only the languages whose key rule the matrix
exercises: Rust, TypeScript, TSX and Python. Rust, TypeScript and TSX also
have real history in the replays, and Python has fixtures only. A function in
JavaScript, Go, Java, Ruby, Swift or Kotlin gets no identity, so it keeps
today's matcher exactly (rule 3 of section 4). Each of those languages joins
the version only with identity fixtures of its own, because each names its
owners differently: a Go method names its receiver, a Java or Kotlin class
nests its methods, and a Ruby method may sit in a `class << self` block.

The census shows how much this covers.

| Repository | Functions | Outside the scope | Identified | Over cc 10 identified | Over the floors identified |
|---|---|---|---|---|---|
| klin | 6,067 | 0 | 4,735 (78%) | 107 of 110 | 382 of 390 |
| TS-A | 2,153 | 95 | 672 (31%) | 23 of 30 | 207 of 232 |
| TS-B | 6,924 | 5 | 2,458 (35%) | 137 of 159 | 461 of 567 |

The census ran `proto5` on klin at `ac861ec2` and on the other two at their
current `HEAD`. "Outside the scope" counts functions in a language
`complexity/1` leaves out, here JavaScript.

Most unidentified TypeScript functions are anonymous callbacks: `it(...)`,
`.map(...)`, `useEffect(...)`. Few of them reach a ceiling. Over the floors,
81% to 98% of the sites are identified. An anonymous site keeps today's
outcome, so the gap costs nothing against today.

### dead-symbols: evaluated, a second user of the envelope

Today the site is the file plus the declaration line, like `complexity`.
F1 applies: `dead-param-rename` and `dead-signature-only` report a
declaration that was dead at the base as `new`. F2 does not apply in the
same way, because a reference counts by name (#49), so two owners' methods
of one name are both dead or both alive.

The candidate key is the inline module nesting, the owner, the declaration
kind and the name. Each comes from the `Declaration` facts that the
structural index already holds for both trees, the base's facts from the
structural cache. A destructuring declaration is ambiguous. So is a trait
method, a trait implementation's method and a TypeScript class member that
shares its name with another in the file, because the facts carry no owner
for them, and the duplicate rule catches them.

No replay measured how often F1 reaches a dead declaration. That replay is
the first step of a `dead-symbols` ticket, and it decides adoption for this
family.

### reachability: keep the current identity

A finding is one file, with a constant text and line 0. The file is the
unit the check judges (SPEC 4.6), and no declaration inside it is the site.
A structural key adds nothing.

### public-api: keep the current identity

The site is the consumer-facing path of an item on a surface. That path is
already the semantic identity: a rename of a public item is a break, and the
check must see it as one. A structural key would hide what the check exists
to report.

### Future test-integrity findings

The #353 prototype keys a test by its module path and function name in Rust,
and by its `describe` titles and test title in TypeScript. That is a key in
this sense. It also numbers a second test with the same key in one file as
` #2`. Under this note's rule that ordinal is not allowed: the two tests are
`ambiguous` with the reason `duplicate`, and they keep the text matching. A
title that holds an interpolation is `computed`.

The same TypeScript title key would also identify most anonymous `it(...)`
callbacks for `complexity`. This note does not propose that: it is a second
key rule, and it needs its own evidence.

## 4. Ambiguity and duplicate-cardinality rules

A site is `ambiguous` when:

- `anonymous`: the site itself has no name and no binding names it, such as a
  callback argument;
- `computed`: the site's name is computed, such as `[key]() {}`;
- `ancestry`: an ancestor on the path is anonymous or computed, such as a
  named function inside a callback;
- `duplicate`: two or more sites of this file on the same side have the same
  key. Every site that shares the key is ambiguous, not only the second.

The duplicate count covers every site of the family in the file, not only the
findings. A key is `identified` only when the syntax names exactly one site.

An ambiguous site never takes part in a key comparison. It keeps exactly
today's matching: the `(file, text)` rank of 16.5, then the body-hash pass.
The proof that this fallback is valid is that it is the shipped contract. An
ambiguous site gets the outcome it gets today, and identity adds no pairing
for it. The rows `property-setter-python`, `duplicate-key-signature`,
`anonymous-callback-ts`, `computed-member-ts` and `nested-under-callback-ts`
show this: each keeps today's outcome.

The same rule covers a pair where one side is identified and the other is
not, or where the versions differ (section 5).

### Pairing rule

For a finding `f` and a `before` entry `e` in one file:

1. When both are identified under one version and the keys are equal, they
   are eligible, whatever their text.
2. When both are identified under one version and the keys differ, they are
   eligible only when their text is equal and neither key persists. When
   either key persists, identity proves that each side has its own
   counterpart, so the text must not pair them.
3. Otherwise, they are eligible when their text is equal, as today.

Among eligible pairs, the rank of 16.5 and the one-to-one greedy pass are
unchanged. The ratcheted values still decide `held` or `worsened`, so the
identity only says which site is compared.

Rule 2 keeps today's outcome where nothing proves a different pairing.
`owner-rename-and-body` and `method-moved-to-other-owner-and-edited` give
today's `held`: the old key is gone, the new key is new, and the text is
equal. An earlier prototype that refused every pair of different keys made
both of these `new`, and that was stricter than today with no proof.

### Move targets

Today a `before` entry may move to another file when its `(file, text)`
group lost entries (4.4). For an identified entry, the rule becomes: the
entry is lost when its key does not persist. This is what fixes F3. An
ambiguous entry keeps the count by text.

### Multiplicity

A key is unique per file per side, or it is not identified. So a key pairing
is one to one by construction, and every duplicate goes to the text matcher,
which pairs one to one as a multiset. One prior site never holds two current
sites. The rows `added-duplicate-occurrence` (Rust) and
`added-duplicate-occurrence-python` add a second site identical to the first.
In both binaries one is `held` and the other is `new`.

### Renames

Product semantics, not the body hash, decide what a rename is. The rule
leaves today's semantics in place:

- a declaration rename with an unchanged body is held by the body-hash pass
  (`rename-same-body`), and a rename with a body edit is `new`
  (`rename-and-body`);
- an owner rename with an unchanged body is held by the body-hash pass
  (`owner-rename-same-body`), and with a body edit it is held by rule 2
  when the text is equal;
- a move to another file with a body edit is `new`
  (`file-move-and-signature`), because the file is outside the key.

## 5. Historical and version compatibility

ADR 0001 has one binary measure both trees in one run, so today no stored
prior finding exists to reinterpret. Three things persist across runs or
builds:

- the finding `id` of 11.2, which the turn record and the journal keep, and
  which `klin stats` counts regressions by;
- the structural cache, which a build reads only when its sources match
  (`SOURCES` in `src/syntax/structural/cache.rs`);
- the accepted list, which a person writes.

The rules:

- **No identity on either side**: today's rules. A finding or entry without
  an identity is never read as structurally identified.
- **Identity on one side only**: today's rules for that pair.
- **Different versions, or a version this build does not know**: today's
  rules for that pair. Two keys of different versions are never compared.
  Equal strings are not enough: a build compares a key only when its version
  is on the build's own list of known versions, so `complexity/999` on both
  sides still falls back to the text.

The matrix tests each rule. The cases `version-none-both-*`,
`version-before-only-*`, `version-after-only-*`, `version-v1-vs-v2-*` and
`version-unknown-both-*` run a parameter rename (where a key would widen the
match), an owner swap (where a key would narrow it) and the F3 move (where a
key changes the move target). All 15 give today's outcome.
- **Changed semantics for one family**: raise that family's version. Other
  families keep theirs.
- **The finding `id` does not change.** It stays the hash of gate, file and
  declaration text, so no journal or turn record from an earlier build
  changes meaning. SPEC 11.5 already forbids merging two ids by resemblance.
  A later `klin stats` could use the identity where two records carry one of
  the same version. This note does not propose that.
- **A cache that holds identities** must name the identity version in what it
  keys on, or drop them. No cache holds findings today.

## 6. Accepted-entry implications

klin's `klin.json` holds no accepted entry and never held one in its
history. TS-A and TS-B have no `klin.json`. No evidence shows that a person
needs to write a structural key.

The proposal keeps the accepted entry as it is: gate, file, text and values.
A key never matches an accepted entry, and rule 3 always applies to one.
Consequences, all shown by the matrix:

- `accepted-and-base-unchanged`: the entry holds the site, as today.
- `accepted-and-base-param-rename`: today the finding is `new` and the entry
  is stale. With identity, the `before` site holds the finding, the entry is
  stale, and `--strict` fails on it as today. So the person is still told to
  edit the entry, and the agent is not blocked on debt it did not add.
- `accepted-param-rename`, with no base site: `new`, and the entry is stale,
  as today.

The JSON would print each finding's identity, so a person can read the key.
A later version MAY let an entry name a key as an alternative to the text.
That needs evidence of an entry that a declaration edit keeps breaking.

## 7. Edit-matrix results

The full table, for both binaries in both modes, is
[`matrix.md`](finding-identity-2026-10-03/matrix.md). The key column is the
prototype's key for each site, by line, before and after the edit. A
`?` key is an ambiguous site and names its reason. The prototype column is
`proto5`. "Today" and "proto" are the whole run. The 15 version cases are in
`matrix.md` only, and section 5 summarizes them.

| Case | Key before → after | Today | Proto | Ambiguity | Fallback still valid |
|---|---|---|---|---|---|
| comment-above | 1 `a` → 3 `a` | held | held | none | text |
| sibling-insert | 1 `a` → 1 `b`; 5 `a` | a held, b new | same | none | text |
| sibling-reorder | 1 `a`; 5 `b` → 1 `b`; 6 `a` | held | held | none | text |
| body-edit | 1 `a` | held | held | none | text |
| signature-only | 1 `a` | held | held | none | body hash |
| signature-and-body | 1 `a` | **new** | held | none | none matches |
| visibility-added | 1 `a` | **new** | held | none | none matches |
| parameter-rename | 1 `a` | **new** | held | none | none matches |
| parameter-rename-ts | 1 `a` | **new** | held | none | none matches |
| parameter-rename-tsx | 1 `Row` | **new** | held | none | none matches |
| rename-same-body | 1 `a` → 1 `b` | held | held | none | body hash |
| rename-and-body | 1 `a` → 1 `b` | new | new | none | none, correctly |
| file-move | src/a.rs:1 `a` → src/b.rs:1 `a` | held | held | none | body hash |
| file-move-and-signature | src/a.rs:1 `a` → src/b.rs:1 `a` | new | new | none | none, by rule |
| nested-signature | 1 `outer`; 2 `outer > inner` | **inner new** | held | none | none matches |
| nested-same-name-two-parents | 1 `p`; 2 `p > inner`; 10 `q`; 11 `q > inner` → 1 `p`; 2 `p > inner`; 8 `q`; 9 `q > inner` | **q > inner held** | worsened 2 to 4 | none | text pairs wrongly |
| two-owners-swap-rust | 4 `impl A > run`; 13 `impl B > run` → 4 `impl A > run`; 11 `impl B > run` | **pass** | B::run worsened | none | text pairs wrongly |
| two-owners-misnamed-rust | 4 `impl A > run`; 13 `impl B > run` → 4 `impl A > run`; 12 `impl B > run` | **names A::run** | names B::run | none | text pairs wrongly |
| two-traits-swap-rust | 3 `impl A as P > run`; 12 `impl A as Q > run` → 3 `impl A as P > run`; 10 `impl A as Q > run` | **pass** | A as Q worsened | none | text pairs wrongly |
| two-owners-swap-python | 2 `A > __init__`; 13 `B > __init__` → 2 `A > __init__`; 9 `B > __init__` | **pass** | B worsened | none | text pairs wrongly |
| two-owners-swap-ts | 3 `A > run`; 13 `B > run` → 3 `A > run`; 11 `B > run` | **pass** | B worsened | none | text pairs wrongly |
| owner-rename-same-body | 3 `impl A > run` → 3 `impl B > run` | held | held | none | body hash |
| owner-rename-and-body | 3 `impl A > run` → 3 `impl B > run` | held | held | none | text, rule 2 |
| method-moved-to-other-owner-and-edited | 4 `impl A > run` → 5 `impl B > run` | held | held | none | text, rule 2 |
| twin-crosses-ceiling | 4 `impl A > run`; 13 `impl B > run` → 4 `impl A > run`; 9 `impl B > run` | **pass** | B::run new | none | text pairs wrongly |
| static-and-instance-ts | 2 `A > static make`; 6 `A > make` | **both new** | both held | none | none matches |
| getter-setter-ts | 3 `A > get value`; 7 `A > set value` | **setter new** | both held | none | none matches |
| property-setter-python | 3 `?duplicate`; 9 `?duplicate` | setter new | same | duplicate | text (today's) |
| duplicate-key-signature | 3 `?duplicate`; 8 `?duplicate` | one new | same | duplicate | text (today's) |
| anonymous-callback-ts | 1 `?anonymous` | new | same | anonymous | text (today's) |
| computed-member-ts | 3 `?computed` | new | same | computed | text (today's) |
| bound-arrow-ts | 1 `f` | **new** | held | none | none matches |
| nested-under-callback-ts | 1 `?anonymous`; 2 `?ancestry` | new | same | ancestry | text (today's) |
| delete-add-replacement | 1 `a` → 1 `b` | b new | same | none | none, correctly |
| same-name-new-body | 1 `a` | held | held | none | text |
| added-duplicate-occurrence | 3 `impl F > new` → 3 `?duplicate`; 8 `?duplicate` | one held, one new | same | duplicate (after) | text multiset |
| added-duplicate-occurrence-python | 1 `h` → 1 `?duplicate`; 7 `?duplicate` | one held, one new | same | duplicate (after) | text multiset |
| accepted-param-rename | none → src/a.rs:1 `a` | new, entry stale | same | none | accepted: text only |
| accepted-and-base-param-rename | 1 `a` | **new**, entry stale | held, entry stale | none | accepted: text only |
| accepted-and-base-unchanged | 1 `a` → 2 `a` | held by the entry | same | none | text |
| twin-moved-to-other-file | src/a.rs:4 `impl A > run`; src/a.rs:10 `impl B > run` → src/a.rs:10 `impl A > run`; src/b.rs:3 `impl B > run` | **B::run new** | both held | none | body hash, by key |
| parameter-rename-js | none | new | same | no identity | text (today's) |
| two-owners-swap-java | none | pass | same | no identity | text (today's) |
| dead-param-rename | n/a | **new** | same (no prototype) | none | none matches |
| dead-signature-only | n/a | **new** | same (no prototype) | none | none matches |
| dead-comment-above | n/a | held | held | none | text |

Bold marks a desired outcome that today's matcher misses. The prototype
gives the desired outcome on every `complexity` row. On the rows where the
desired outcome needs a key that the syntax does not give
(`property-setter-python`), it keeps today's outcome.

`parameter-rename-js` and `two-owners-swap-java` are the negative cases for
the language scope: no site gets a key, and the outcome is today's, the
false pass of the Java owner swap included.

## 8. Performance evidence

The producer adds no parse and no tree walk. It climbs the ancestors of a
function node that the existing `complexity` walk already holds, reads name
fields, and builds one string. It needs no persistent AST and no external
process. Its census for `persists` is the function list that both sweeps
already build.

Measurements, on an Apple silicon Mac with no other load:

- **klin clone, 20 changed `.rs` files, `gate --gate complexity --changed`,
  warm, `hyperfine` 40 runs**: baseline 468.0 ± 3.4 ms, prototype
  468.6 ± 3.8 ms, prototype with the identity turned off 468.8 ± 2.5 ms. The
  gate's own `ms`, median of 30: 110 and 110.
- **klin, whole `complexity` run**, both trees whole, about 6,000 functions
  each, median of 15 runs of the gate's `ms`: prototype 918, prototype with
  the identity off 904. That is an upper bound near 14 ms for about 12,000
  functions, about 1.2 µs per function.
- **`structural_300k` `warm20`**, 20 changed files, 5 iterations a row,
  three rows a binary, alternating, from this branch's
  `tests/performance.rs` with `KLIN_BIN`:
  [`perf-300k-warm20.txt`](finding-identity-2026-10-03/perf-300k-warm20.txt).

  | Binary | Hook medians (ms) | `complexity_ms` |
  |---|---|---|
  | baseline | 859, 845, 846 | 16, 16, 16 |
  | `proto4` | 843, 846, 840 | 16, 17, 16 |

  The difference is inside the row's noise, which is above 10 ms.

`proto3` cost about 14 ms more in the same row. That cost was in
`reachability` (8 to 9 ms) and `dead-symbols`, which carry no identity: the
`proto3` matcher grouped every gate by file and built text maps per file.
`proto4` sends every gate whose findings and entries carry no identity
through today's matcher, unchanged, and at 300k the cost is gone. An
implementation must keep that fast path.

- **`structural_1m` `warm20`**, run at the owner's request after the rest of
  this note, the same way: 1,033,827 lines, 20 changed files, three rows a
  binary, alternating:
  [`perf-1m-warm20.txt`](finding-identity-2026-10-03/perf-1m-warm20.txt).

  | Binary | Hook medians (ms) | `complexity_ms` | `dead-symbols_ms` | `reachability_ms` |
  |---|---|---|---|---|
  | baseline | 1342, 1321, 1326 | 27, 27, 27 | 554, 552, 553 | 298, 297, 299 |
  | `proto4` | 1349, 1339, 1326 | 28, 28, 28 | 565, 561, 556 | 303, 304, 304 |

The 1M row does not repeat the 300k result:

- The identity's own cost is 1 ms: `complexity_ms` is 1 ms higher in each
  pair.
- The hook is 8 ms slower on the mean of the three medians, and 13 ms slower
  on their median. The median is above the preferred 10 ms.
- Most of that is in `dead-symbols` (3 to 11 ms) and `reachability` (5 to
  7 ms), which carry no identity and take today's matcher unchanged. The
  first candidate cause was the `proto4` check that scans every finding and
  entry for an identity before it picks the matcher. The `proto5` rows below
  make that unlikely.

`proto5` removes the scan: every gate but `complexity` calls today's judge
directly, and `complexity` splits its files by language before it judges.
Measured after that change, alternating, 5 iterations a row:

- `structural_300k` `warm20`, three rows a binary: baseline hook medians 848,
  859 and 851 ms, `proto5` 852, 851 and 858 ms. `complexity_ms` 16 against 16
  or 17.
- A generated JavaScript-only tree, 5,000 files and 560,000 lines, `cc 0` so
  every function is a finding, 20 changed files, `hyperfine`: the changed run
  takes 190.6 ± 3.7 ms on the baseline and 190.7 ± 5.3 ms on `proto5`. The
  whole run, about 100,000 findings, takes 4.883 ± 0.028 s against
  4.911 ± 0.033 s, a difference of about one σ.

- `structural_1m` `warm20`, three rows a binary, also at the owner's
  request:

  | Binary | Hook medians (ms) | `complexity_ms` | `dead-symbols_ms` | `reachability_ms` |
  |---|---|---|---|---|
  | baseline | 1367, 1334, 1328 | 27, 27, 27 | 561, 542, 559 | 310, 301, 303 |
  | `proto5` | 1369, 1345, 1339 | 28, 28, 28 | 562, 556, 560 | 310, 309, 306 |

At 1M, `proto5` repeats the `proto4` result:

- The identity's own cost is 1 ms again: `complexity_ms` is 1 ms higher in
  each pair.
- The hook is slower by 2, 11 and 11 ms in the three pairs: 8 ms on the mean
  and 11 ms on the median, against 8 and 13 ms under `proto4`.
- Under `proto5`, `dead-symbols` and `reachability` run today's matcher code
  with no scan, and they are still 0 to 14 ms slower in some pairs. So the
  scan was probably not the cause. A different code layout in the binary is
  one remaining explanation. These rows do not prove it.
- The baseline's own hook median ranges from 1328 to 1367 ms over these
  rows, a spread of 39 ms, which is larger than the gap. But the prototype
  was slower in all six pairs of `proto4` and `proto5`, so the gap is not
  shown to be noise either.

So the identity is a candidate for the ordinary Stop at about 1 ms. A gap of
about 10 ms in the whole hook remains at 1M/20, and its cause is not proven.
An implementation must show at 1M/20 that it costs what today costs
(section 9).

## 9. Smallest implementation boundary

If a person admits this, the first ticket is `complexity` alone:

1. `ratchet::Finding` gains `identity: Option<Identity>`, a typed field and
   not a member of `values`. `Identity` holds the version and either the key
   with `persists`, or the ambiguity reason.
2. A family bit alone cannot keep an unsupported language on today's
   matcher, because `complexity` is one family over many languages. The
   dispatch is two-level, as in `proto5`:
   - a gate whose family carries no identity calls today's judge, with no
     scan of its findings;
   - `complexity` splits its findings and entries by the language of the
     file. A file in a language outside `complexity/1` takes today's first
     pass, by file and text, exactly. A file inside it takes the pairing
     rule and the move-target rule of section 4. One body-hash pass then
     runs over what both passes leave, as today's second pass does.

   At 1M/20 the whole hook under `proto4` and `proto5` was 8 to 13 ms
   slower, the identity's own share was 1 ms, and the cause of the rest is
   not proven (section 8). `proto5` removed the scan and kept the gap. So
   the implementation must show at 1M/20 that a gate with no identity, and a
   `complexity` run over unsupported languages only, cost what they cost
   today, before it ships.
3. `complexity` computes the key in its existing walk, for Rust,
   TypeScript, TSX and Python only, marks duplicates per file, and sets
   `persists` from the function lists of both sweeps.
4. The JSON finding and its `matched` record print the identity. The `id`
   does not change.
5. The CLI tests are the `complexity` rows of the matrix, written as cases
   of `tests/complexity.rs`.

Out of the first ticket: accepted entries, `dead-symbols`, the journal and
`klin stats`, and any cache. `dead-symbols` is a second ticket that starts
with its replay.

## 10. Proposed SPEC changes

- **4.4 Site**: a check MAY attach a versioned identity to each site. Add the
  envelope, the ambiguity reasons, the duplicate rule over every site of the
  file, the three pairing rules, and the move-target rule for an identified
  entry. State that an ambiguous site, a site with no identity, a pair under
  different versions, and a key of a version the build does not know keep
  the text rules.
- **4.5 Finding**: the optional `identity` field.
- **4.8 Accepted entry**: an entry carries no identity, and the text rule
  always applies to it.
- **8.2.1 complexity**: the key rule of section 3, the reasons, and the
  languages `complexity/1` covers.
- **11.2 JSON**: the `identity` object on a finding and on `matched`. The
  `id` stays as it is.
- **16.5**: the eligibility step before the rank, and the inserted-twin and
  moved-twin examples with keys.
- **17**: the matrix rows.
- **ADR**: a new ADR that amends ADR 0008. ADR 0008 makes the file and the
  declaration line the primary key, matched before any body is read. The new
  ADR would make an identified key the primary key where both sides carry one
  of the same version, with the declaration line as the fallback. It keeps
  ADR 0008's refusal of a metric in the key.

## Limits

- Two of the three repositories belong to one owner, and the replays cover
  Rust and TypeScript only. Python appears in the matrix only. No fixture or
  census covers JavaScript, Go, Java, Ruby, Swift or Kotlin, so
  `complexity/1` leaves them out (section 3).
- The agent that wrote the prototype also classified the differences. The
  machine check is narrow: is the finding's text absent from the parent's
  file? The 11 rows where the prototype is stricter were read by hand.
- The replays ran `--changed` only. The matrix shows the whole run agrees.
- The exposure replay inflates counts by design. The floor replay is the one
  that shows real `complexity` verdicts, and only one changed in 600
  commits. The replays ran no other gate, so they show nothing about a whole
  stop.
- F2 appeared only in the exposure replay.
- No `dead-symbols` replay ran.
- The 1M rows show 8 to 13 ms more for the whole hook under `proto4` and
  `proto5`, mostly in gates with no identity, and this note did not find the
  cause.

## Decision

**Adopt versioned optional finding identity.**

The proving family is `complexity`, in Rust, TypeScript, TSX and Python. Its
key is the named path of section 3, compared only within one file. Its other
languages keep today's matcher until each has identity fixtures. The matching
rules are the three pairing rules and the move-target rule of section 4. A
site is ambiguous when it is anonymous, computed, under an anonymous or
computed ancestor, or one of two sites with the same key in its file. An
ambiguous site, a site with no identity, and a pair under different versions
keep today's text and body-hash rules exactly. The version is per family, a
key is compared only under a version the build knows, a pair under two
versions is never compared, and the finding `id` does not
change. Accepted entries stay keyed by text.

The shared envelope is useful because three families need the same version,
ambiguity and `persists` semantics with different keys: `complexity` now,
`dead-symbols` after its replay, and test-integrity findings when they ship.
`reachability` and `public-api` keep their current identities, which are
already more truthful than a structural key.

This result does not authorize an implementation.
