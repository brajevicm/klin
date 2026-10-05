# Canonical duplication normalization: design and handover, 2026-10-05

This note belongs to #480 under #478. It is the research design and the
handover for the agent that runs the research. It holds no results yet.

The ratchet model is settled in `docs/duplication-ratchet-lineage-2026-10-05.md`
(#479). This note decides only what one canonical fingerprint means.

Status: design reviewed once by an adversarial pass. The fixes from that pass
are already in this design. Nothing below has been measured.

## 1. Scope

In scope:

- a standalone research prototype that makes canonical token streams;
- four normalization profiles, P1 to P4, with the variants in section 5;
- per-language calibration of Rust and TypeScript/TSX;
- a labeled corpus and a frozen decision rule;
- a frozen candidate that #481 can attack.

Out of scope, from the issue:

- production code in `src/`;
- partial or block clones (#483);
- agent repair runs;
- a final BLOCK decision;
- performance work (#482). The prototype records only a rough cost.

## 2. Prototype layout

Put everything under `docs/duplication-normalization-2026-10-05/`. Follow the
layout of `docs/design-conformance-2026-10-02/`:

```text
prototype/        standalone crate, own [workspace], not in klin's workspace
fixtures/         synthetic pairs, one directory per case
census/           collision census outputs, one TSV per repository
labels/           blind label worksheets and filled labels
expected.tsv      expected result per fixture pair and profile
results.tsv       measured rows (section 8)
probe.sh          rebuilds every result from pinned inputs
```

Pin the crate to klin's grammar versions: `tree-sitter = "0.27.0"`,
`tree-sitter-rust = "0.24.2"`, `tree-sitter-typescript = "0.23.2"`. Add `sha2`.
Build with its own `CARGO_TARGET_DIR`, as the other probes do.

Do not import klin as a library. Copy the small pieces you need, for example the
test-range logic in `src/syntax/convention.rs` (`cfg_test_ranges`,
`test_module_ranges`). Record each copied piece and its source commit.

Make all output deterministic: `LC_ALL=C`, sorted rows, no timestamps in
result files.

## 3. Pipeline

For each source file:

1. Parse with the grammar for the file extension. Use `typescript` for `.ts`,
   `tsx` for `.tsx`. Both map to language `ts`.
2. Find eligible units (section 4).
3. For each unit, collect leaf tokens in source order. Skip comments.
4. Classify each identifier leaf with the binding walker (section 6).
5. Write one canonical stream per profile and variant (section 5).
6. Hash each stream with sha256. Keep the stream text for an equality check.
7. Group units by `(language, profile, variant, hash)`. Confirm that the
   streams in each group are equal.
8. Write one row per group and per threshold.

Use the **text** of each leaf token, never the node kind. The TS and TSX
grammars give different node kinds for some of the same code. The text keeps
TS and TSX as one identity. Fixture `tsx-same-as-ts` checks this.

The token count of a unit is the number of non-comment leaf tokens, with
punctuation included. The count is equal across P1 to P4, because the profiles
change only spellings. The count is not comparable between Rust and TS. Each
language gets its own threshold.

## 4. Eligible units

Rust:

- `function_item` with a body: free functions, inherent methods, trait impl
  methods and trait default methods;
- not trait method declarations without a body;
- not closures.

TypeScript/TSX:

- E1: `function_declaration`, `generator_function_declaration`,
  `method_definition` with a body;
- E2: E1 plus `arrow_function` and `function_expression` that are the direct
  value of a `const` declarator or of a class field;
- not overload signatures, `declare` members or `abstract` methods;
- not anonymous callbacks, for example an arrow passed as an argument.

Measure E1 and E2 separately. Admit E2 only if it adds positives without a
blocking hard negative.

Nested functions:

- A nested eligible item is its own unit.
- Inside its parent's stream, the nested item appears as P1 tokens. Locals of
  the parent are not normalized inside it, because Rust items do not capture.
- A TS nested function is a closure. Walk it as a child scope of the parent
  (section 6). It is its own unit only if it meets E1 or E2.

Scope exclusions:

- Mark test code with copied klin test-range rules and the test path
  conventions. Do the census with test code both in and out. The candidate
  default is out.
- Skip generated and vendored paths with the same rules klin already uses for
  scope. Record each rule used.

## 5. Profiles and variants

Every profile drops comments and whitespace. Keywords and punctuation always
keep their text.

| Profile | Locals and parameters | Own function name | Anchors | Literals |
|---|---|---|---|---|
| P1 exact | kept | `SELF` | kept | kept |
| P2 alpha locals | `L0`, `L1`, … | `SELF` | kept | kept |
| P3 alpha + literals | as P2 | `SELF` | kept | selected classes normalized |
| P4 broad | `ID` | `ID` | `ID` | `LIT` |

The own function name is never part of the stream. Each use of the own name
in the body becomes `SELF`, so a renamed recursive function still matches. In
a method, only `self.name(` or `this.name(` with the same name counts as an
own-name use. Other uses keep their text.

The P2 slots number the bindings in order of first binding occurrence. These
are de Bruijn-style indices. `price + price` gives `L0 + L0`, and
`left + right` gives `L0 + L1`.

Shorthand fields: write a shorthand field in its expanded form.

- Rust `Point { x, y }` in an expression or a pattern gives
  `Point { x : L0 , y : L1 }`.
- TS `{ x }` in an object literal or a pattern gives `{ x : L0 }`.
- The field name stays an anchor. The binding gets a slot.
- Then `Point { x }` and `Point { x: a }` give the same stream.

Variants. Measure each one separately:

- **M-strict / M-aware (Rust macros).** In M-strict, all identifiers inside a
  macro `token_tree` keep their text. In M-aware, such an identifier becomes
  its slot only when all of these are true:
  1. it has the same text as a local in scope at that point;
  2. the next token is not `!` or `::`;
  3. the previous token is not `.` or `::`.

  M-aware may become a candidate only if it adds no blocking hard negative.
  Record how many units contain a local name inside a macro under M-strict.
- **O-none / O-type (owner).** O-type adds the enclosing `impl` type or class
  name in front of the stream. O-none does not. See the label policy in
  section 9.
- **P3 literal classes.** Measure each class separately: `num`, `str`, and
  `num+str`. String literals keep their text in these places:
  - object keys, computed member keys and import paths;
  - comparisons with `===`, `==` or `!=`;
  - Rust `match` arm patterns;
  - the format string of a macro;
  - discriminant fields such as `type:` or `kind:`.

  Admit a class only if it catches a required positive that P2 misses, and
  only if it adds no blocking hard negative. Hard negative
  `validation-constants` will likely reject literal normalization. Record the
  result either way.

## 6. Binding walker

This walker covers one function at a time and uses only lexical scope. It
is not a name resolver. If correct output needs compiler-scale resolution,
record that as evidence against P2 and fall back toward P1 (see #480).

Rule for unclear cases: **keep the original text**. Exclude a unit only when
keeping the text would still give a wrong stream. Exclusion is a way for an
agent to avoid the gate (section 11), so use it as little as possible. Record
each exclusion with its reason.

Every identifier leaf gets exactly one class:

| Class | Output in P2/P3 | Examples |
|---|---|---|
| binding | new slot | `let x`, parameter `a`, `catch (e)` |
| local-ref | slot of the binding found in scope | `x + 1` |
| own-name | `SELF` | recursive call |
| anchor | original text | field, member, type, path, macro name, global, import |
| kept-unclear | original text | identifier inside a Rust macro (M-strict) |

Rust rules:

- Bindings: parameter patterns, `let` patterns, `for` patterns, `match` arm
  patterns, `if let` and `while let`, and closure parameters.
- Each block, closure, match arm and `if let` body opens a new scope.
- A new `let` shadows the old binding with a new slot.
- In a pattern, an `identifier` that starts with an uppercase letter is an
  anchor: a const, a unit struct or an enum variant. Example: `None`.
- In a pattern, a lowercase `identifier` is a binding. `ref`, `mut` and
  `x @ pat` bind `x`.
- Keep these as anchors: `self`, `Self`, `field_identifier`,
  `type_identifier`, every segment of a `scoped_identifier`, lifetimes,
  labels, and macro names.
- An identifier that does not resolve to a local in scope is an anchor
  (global, const, function or static).
- Nested `fn`, `struct`, `impl` and `mod` items: P1 tokens, no parent scope
  (section 4).
- Macro token trees: see M-strict / M-aware.

TypeScript rules:

- Bindings:
  - parameters, including rest and default parameters;
  - destructuring patterns: in a `pair_pattern`, the key is an anchor and the
    value binds; a shorthand pattern is expanded (section 5);
  - `let`, `const` and `var`;
  - `catch` parameters, `for…in` and `for…of` bindings;
  - names of nested function and class declarations.
- Before the walk, hoist the `var` bindings and nested function declarations
  to the top of their function scope. A `let`, `const` or `class` binding
  belongs to its block.
- Each block, arrow and function expression opens a new scope.
- Keep these as anchors: `this`, `super`, `property_identifier`,
  `type_identifier`, the names of generic type parameters, JSX element and
  attribute names, import names, labels, and identifiers that do not resolve
  in scope.
- `typeof x` inside a type position: keep the text.
- `eval(`, `with`, and `arguments`: keep every token in the unit as P1 text
  and mark the unit `unclear`. Do not exclude the unit.

Cross-check for TS: `tree-sitter-typescript-0.23.2/queries/locals.scm` exists.
It covers highlighting only, so it is coarse. Use it as a second oracle on the
fixtures: list each identifier where the walker and `locals.scm` disagree, and
explain each one. `tree-sitter-rust-0.24.2` ships no `locals.scm`, so Rust has
no second oracle.

Unit tests: give the prototype ordinary `#[test]` tests for the walker over
the construct fixtures in section 7. This is research code, so the CLI-only
test rule of klin does not apply to it.

## 7. Corpus

### Tier 1: synthetic pairs

Each case is a directory with `a.<ext>` and `b.<ext>` and an expected result
for each profile in `expected.tsv`. Write each case in Rust and in TS. Make each
positive large enough to clear 150 tokens, plus one copy of each positive at
about 70 tokens to show where the threshold cuts.

Required positives (must collide under the candidate):

- `exact-copy`;
- `format-comment-only`;
- `function-rename`, including `function-rename-recursive`;
- `local-rename` (consistent rename of parameters and locals);
- `local-rename-shorthand` (renamed copy that changes the shorthand `{x}` to
  `{x: a}`);
- `local-rename-macro` (Rust: renamed local used inside `format!`; must
  collide under M-aware, records a miss under M-strict);
- `two-added-together`;
- `cross-file-copy`;
- `legacy-plus-third` (two base copies, one new third copy);
- `tsx-same-as-ts`.

Required hard negatives (must not collide under P2/P3):

- `same-skeleton-different-api`;
- `crud-wrappers`;
- `serializers-different-schema`;
- `validation-constants`;
- `react-boilerplate` (TSX);
- `trait-required-methods` (Rust) and `interface-required-methods` (TS);
- `error-adapters`;
- `generated-protocol`;
- `table-dispatch`;
- `test-helpers`;
- `glaredb-executor` (the same executor scaffold with a different member or
  API; model it on GlareDB#3633);
- `owner-only-differs` (the same body in two `impl` blocks or classes; result
  depends on O-none / O-type).

Anchor tests (#480 AC on API/member/type preservation). Each pair differs in
exactly one anchor. It must not collide under P1 to P3 and must collide under
P4:

- `anchor-method`, `anchor-field`, `anchor-type`, `anchor-enum-variant`,
  `anchor-imported-fn`, `anchor-macro` (Rust), `anchor-jsx-element` (TSX).

Binding construct tests, both languages where the construct exists:

- `shadowing`, `destructuring`, `closure-capture`, `match-pattern-binding`
  (Rust), `uppercase-pattern-const` (Rust), `var-hoisting` (TS),
  `catch-binding` (TS), `bound-arrow`, `anonymous-callback`,
  `member-vs-local-same-name` (`x.x`), `unclear-eval` (TS).

### Tier 2: existing material, before any fresh sampling

1. `docs/finding-identity-2026-10-03/` (#425): `parameter-rename-ts`,
   `parameter-rename-tsx`, `bound-arrow-ts`, `anonymous-callback-ts`,
   `getter-setter-ts`, `static-and-instance-ts`, `computed-member-ts`,
   `nested-under-callback-ts`, `two-owners-swap-ts`.
2. `docs/design-conformance-2026-10-02/fixtures/payments-rs` and
   `payments-ts` (#355).
3. Natural cases from #355, through `git archive`:
   - `GlareDB/glaredb#3633`, `8001afa4` to `44da2223`;
   - `karakeep-app/karakeep#1723`, `f8ae9866` to `87b39726`.
4. The #357 pilot rows coded `reuse` in
   `docs/phenotype-pilot-2026-10-02/codes.tsv`: the two cases above and
   `langgenius/dify-official-plugins#1422`. Get commits from
   `docs/phenotype-pilot-2026-10-02/selection.json`. Check the language of
   dify before use. It may be Python, which is out of scope.
5. #343 and #361 material: search `docs/labeling-2026-09-21.md` and the #361
   run records for copied implementations. Take only the copies that a
   reviewer or a label marked.

For each tier-2 change, run the prototype on the base and the head. Report
groups whose multiplicity at the head is higher than at the base.

### Tier 3: collision census (precision)

This tier fixes the largest gap of the first design: too few natural
negatives.

Run the prototype over whole **base** trees and report every collision group
at 60 tokens or more:

- every repository in `docs/phenotype-pilot-2026-10-02/selection.json` that
  has Rust or TS;
- GlareDB at `8001afa4`;
- karakeep at `f8ae9866`;
- klin's own `src/` at the baseline commit.

These are not agent changes. They do not count against the cap of 30.

Before the census, fix in `labels/protocol.md` a minimum number of labeled
P2 groups for each language. Suggested minimum: 50 groups. If a language has
fewer groups, its strength can be at most "BLOCK candidate, insufficient
evidence".

If a language produces more groups than can be labeled, take a seeded random
sample. Record the seed.

### Tier 4: fresh holdout, only if needed

Use a fresh natural holdout only if tiers 1 to 3 cannot separate two
candidate profiles. Cap: 30 agent changes. Freeze the selection before the
prototype runs on it.

## 8. Measurements

`results.tsv` has one row for each combination of language, profile, variant,
eligibility (E1/E2) and threshold.

Thresholds: 40, 60, 80, 100 and 150. After a threshold is selected, add rows
at the selected value plus and minus 10 and 20, so that a brittle cutoff
shows.

Columns:

- `positives_caught`, `positives_required`;
- `hard_negatives_collided` (tier 1);
- `census_groups`, `census_copy`, `census_noncopy`, `census_unlabeled`;
- `units_eligible`, `units_unclear`, `units_excluded`;
- `macro_local_units` (Rust, M-strict);
- `walker_us_p50`, `walker_us_p95`, for each unit. This is a rough cost for
  #482, not a gate.

## 9. Labels

Write `labels/protocol.md` before the first census run, and commit it before
any label.

Label each group with exactly one label:

| Label | Meaning |
|---|---|
| `copy` | meaningful copied implementation; a reviewer would ask for reuse or extraction |
| `boilerplate` | framework or language ceremony |
| `required-shape` | a trait, interface or protocol requires the shape |
| `generated` | generated or protocol-style code |
| `test` | test code or a test helper |
| `distinct` | same shape, different responsibility |

Owner policy, fixed now: the same body in two different owner types is `copy`
when a shared helper or default method could hold it. It is `required-shape`
when each owner must provide its own implementation for a trait or interface.
O-type and O-none are both measured against this policy.

Blind labeling:

- Put the groups of all profiles into one worksheet. Remove profile and
  variant names.
- Shuffle with a recorded seed.
- Include groups that only P4 finds. If the labeler marks many of them
  `copy`, the labeler is too generous. Record the rate.
- Keep the labeler separate from the code that maps rows back to profiles.

The labels are agent-drafted. Say so in the result note.

## 10. Frozen decision rule

Commit this rule before measurement. Do not change it after you see results.

A **blocking hard negative** is a collision group at or above the threshold,
in production scope, with any label other than `copy`.

For each language, choose the least aggressive profile and variant, in the
order P1 < P2 < P3, that meets the rule for its strength:

- **BLOCK candidate**, when all of these are true:
  1. it catches every required positive at or above the threshold;
  2. it has zero blocking hard negatives in tier 1, in the census and in any
     holdout;
  3. the census for that language meets the minimum group count.
- **BLOCK candidate, insufficient evidence**: conditions 1 and 2 hold, but
  condition 3 does not.
- **REVIEW**: the census has more `copy` groups than other groups, and
  `glaredb-executor` does not collide.
- **reject**: none of the above.

P4 is a comparator only. It is never a candidate.

Rust and TS are judged independently. They may get different profiles,
variants and thresholds.

## 11. Output for #481

The result note must contain one frozen candidate for each language. Each
candidate states:

- eligible units;
- the normalization contract (profile, variant, literal classes, owner);
- the semantic anchors preserved;
- the minimum size;
- unsupported and unclear behavior;
- positive catches and hard-negative results;
- natural examples from tiers 2 and 3;
- known blind spots;
- strength (BLOCK | REVIEW | reject);
- an **evasion catalog**: the cheap edits that change the fingerprint. List
  at least these:
  - a no-op statement;
  - statement reordering;
  - one changed literal (when literals are kept);
  - shorthand that is not expanded (when applicable);
  - wrapping the body in a macro;
  - adding `arguments` or `eval` (TS).

  Measure each evasion on the `local-rename` fixture.

## 12. Handover

### Start state

- Branch: `issue-480-normalization-research`. It holds only this note.
- Baseline: `b61ac917` on `main`.
- Nothing is measured. No issue comment has been posted.

### Steps

1. Read #480, #478, `docs/duplication-ratchet-lineage-2026-10-05.md` and this
   note.
2. Write `labels/protocol.md` and the decision rule in section 10. Commit them
   alone, before any measurement. The first-measurement commit must come
   after this commit.
3. Write the tier-1 fixtures and `expected.tsv`. Commit.
4. Build the prototype: parse, eligibility, token streams, then the walker,
   then the profiles and variants. Add walker unit tests over the construct
   fixtures. Run the TS `locals.scm` cross-check and explain every
   difference.
5. Make every tier-1 row in `expected.tsv` pass, or record why a row fails.
   Do not change an expected value to make a row pass without a written
   reason.
6. Run tier 2. Export trees with `git archive` into a temp directory. Do not
   commit third-party source trees. Commit only the derived TSV rows and short
   excerpts.
7. Run the tier-3 census, then blind labeling.
8. Fill `results.tsv`. Select thresholds. Add the rows around each selected
   threshold.
9. Use tier 4 only if section 7 allows it.
10. Write the result section and the frozen candidates (section 11) into this
    note, below a `## Results` heading.
11. Run `/code-review` on the branch.
12. Open a PR with `Closes #480`. Run the `humanizer` skill on the PR body
    first.
13. Draft the #478 update comment with the winning candidates. Run the
    `humanizer` skill on it. **Ask the user before posting it.**

### Rules

- Never commit to `main`.
- Do not edit `klin.json`, the hooks, or the `accepted` list.
- Do not edit `src/`. If the pre-commit gate blocks a docs-only commit,
  stop and ask the user.
- Do not run the full Rust suite or benchmarks. The user runs them.
- Keep CI runs to a minimum (CI budget).
- Never change the decision rule or the labels after seeing results. If the
  rule is wrong, record the problem and ask the user.
- Never guess a local binding for a candidate fingerprint. Keep the text.

### Known risks

- P2 may need resolution beyond lexical scope in Rust (macros, `use` inside a
  function body). If so, record it as evidence against P2 for Rust and fall
  back toward P1. Do not grow the walker into a resolver.
- The census may be dominated by `required-shape` groups in Rust trait
  implementations. If so, the O-type variant and the owner policy decide the
  result. Report both.
- dify#1422 may be out of language scope.
- The labels are agent-drafted. The user may want to review a sample before
  the result note claims a strength.
