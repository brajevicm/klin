# Canonical duplication normalization: design and handover, 2026-10-05

This note belongs to #480 under #478. It is the research design and the
handover for the agent that runs the research. It holds no results yet.

The ratchet model is settled in `docs/duplication-ratchet-lineage-2026-10-05.md`
(#479). This note decides only what one canonical fingerprint means.

Status: design reviewed by two adversarial passes. The second pass is the PR
#488 review. The fixes from both passes are in this design. Nothing below has
been measured, except the three semantic checks in section 13.

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

The size of a unit is the number of non-comment leaf tokens in the source,
with punctuation included, counted before any transform. Shorthand expansion
(section 5) adds tokens to a stream, so the stream length is not the size.
The size is equal for every profile and variant of one unit. A collision group
qualifies for a threshold when its smallest unit is at or above it. The size
is not comparable between Rust and TS. Each language gets its own threshold.

A unit whose subtree contains an `ERROR` node or a missing node is `unclear`.
It is fingerprinted and reported, but it can never be part of a blocking
group. Report the count for each language.

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

| Profile | Locals and parameters | Anchors | Literals |
|---|---|---|---|
| P1 exact | kept | kept | kept |
| P2 alpha locals | `L0`, `L1`, … | kept | kept |
| P3 alpha + literals | as P2 | kept | selected classes normalized |
| P4 broad | `ID` | `ID` | `LIT` |

The declaration name of the unit is never part of the stream, in every
profile. The unit is its parameters, return type and body. This is a choice
of unit boundary, not a rewrite, so P1 stays exact.

Own-name switch `N` (off or on) is a separate transform, measured with every
profile. With `N` on, a bare call of a free function's own name becomes
`SELF`, but only when no local in scope has that name. `self.name(`,
`this.name(` and every other member use keep their text in every profile,
because a member name is an anchor. A renamed recursive method therefore does
not match. Record this as a blind spot.

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

- **M-strict / M-std (Rust macros).** tree-sitter-rust gives a macro's
  arguments as a flat `token_tree` of `identifier` and punctuation leaves
  (section 13, check 3). A token-neighbor rule cannot tell a closure
  parameter or a struct field from a local, so this design does not use one.
  - M-strict: every identifier inside a macro `token_tree` keeps its text.
    This is the conservative default.
  - M-std: for a closed list of std macros whose arguments the std docs define
    as expressions, the prototype parses the argument text again as Rust
    expressions. The list is: `format!`, `print!`, `println!`, `eprint!`,
    `eprintln!`, `write!`, `writeln!`, `panic!`, `assert!`, `assert_eq!`,
    `assert_ne!`, `debug_assert!`, `debug_assert_eq!`, `debug_assert_ne!` and
    `vec!`. The walker then classifies the parsed subtree with the enclosing
    scope, like any other expression. Rules:
    1. Wrap the argument text as `[ ARGS ]` for `vec!` and as `f( ARGS )` for
       the others. If the parse has an `ERROR` or missing node, the whole
       macro keeps M-strict text and the unit is `unclear`.
    2. In the format family, a top-level argument of the form `name = expr`
       is a named format argument. `name` is an anchor. `expr` is walked.
    3. The format string keeps its text in both variants. An inline capture
       such as `"{x}"` is not normalized, so a renamed copy that uses inline
       capture does not match. Record this as a blind spot, with a count.
    4. Every other macro keeps M-strict text.

  M-std may become a BLOCK candidate, because the std docs define these
  arguments as expressions and the walker resolves them with the same rules
  as ordinary code. Record how many units contain a local name inside a macro
  under M-strict.
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
| own-name | `SELF` with switch `N` on, else original text | bare recursive call of a free function |
| anchor | original text | field, member, type, path, macro name, global, import |
| kept-unclear | original text | identifier inside a Rust macro (M-strict) |

Rust rules:

- Bindings: parameter patterns, `let` patterns, `for` patterns, `match` arm
  patterns, `if let` and `while let`, and closure parameters.
- Each block, closure, match arm and `if let` body opens a new scope.
- A new `let` shadows the old binding with a new slot.
- In a pattern, an `identifier` that starts with an uppercase letter keeps
  its text, for example `None`. This direction is always safe: if the name is
  in fact a binding, the cost is only lost recall.
- A bare lowercase `identifier` in a pattern is not certain to be a binding,
  in any pattern position. rustc resolves it to a unit struct or a const when
  one with that name is in scope, also in `let`, parameter, closure and `for`
  patterns, if the pattern is irrefutable (section 13, check 1). The walker
  treats it as a binding only when all of these are true:
  1. no item, `use` import or function-local item anywhere in the file has
     that name;
  2. the file has no glob import (`use …::*`);
  3. the file has no macro invocation at item level, which could define an
     item with that name.

  Otherwise the identifier keeps its text and the unit is `unclear`. Report
  the `unclear` rate for each pattern position. A high rate is evidence
  against P2 for Rust, not a reason to relax the rule. `ref`, `mut` and
  `x @ pat` follow the same rule for `x`.
- Keep these as anchors: `self`, `Self`, `field_identifier`,
  `type_identifier`, every segment of a `scoped_identifier`, lifetimes,
  labels, and macro names.
- An identifier that does not resolve to a local in scope is an anchor
  (global, const, function or static).
- Nested `fn`, `struct`, `impl` and `mod` items: P1 tokens, no parent scope
  (section 4).
- Macro token trees: see M-strict / M-std (section 5).

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
- `function-rename`;
- `local-rename` (consistent rename of parameters and locals);
- `local-rename-shorthand` (renamed copy that changes the shorthand `{x}` to
  `{x: a}`);
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

Optional recall probes. They are reported but are not required, so no
variant is forced by them:

- `local-rename-macro` (Rust: renamed local passed as an argument to
  `format!` and `assert_eq!`);
- `local-rename-inline-capture` (Rust: renamed local in `"{x}"`; expected to
  miss in every variant);
- `function-rename-recursive` (expected to match only with switch `N` on);
- `method-rename-recursive` (expected to miss in every variant).

Binding construct tests, both languages where the construct exists:

- `shadowing`, `destructuring`, `closure-capture`, `match-pattern-binding`
  (Rust), `uppercase-pattern-const` (Rust), `var-hoisting` (TS),
  `catch-binding` (TS), `bound-arrow`, `anonymous-callback`,
  `member-vs-local-same-name` (`x.x`), `unclear-eval` (TS),
  `lowercase-unit-struct-pattern` (Rust: `struct s; let s = s;` must be
  `unclear`), `macro-closure-arg` (Rust: `vec![|a| a + 1]`),
  `macro-struct-field` (Rust: `vec![Foo { x: y }]`), `format-named-arg`
  (Rust: `format!("{v}", v = x)`), `recovered-parse` (both: a unit with an
  `ERROR` node).

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

For each tier-2 change, run the prototype on the head. Report the
co-canonical groups that contain at least one unit the change added or
modified. Label each group like a census group (section 9).

Tier 2 answers only "which units share a canonical identity, and is each
group a real copy?". It does not claim that a change created a duplication
regression. That question belongs to the lineage model of #479
(`certain_bundles(H)`, `certain_regressions(H)`), which is out of scope here.

### Tier 3: collision census (precision)

This tier fixes the largest gap of the first design: too few natural
negatives.

Run the prototype over whole **base** trees and report every collision group
at size 60 or more:

- every repository in `docs/phenotype-pilot-2026-10-02/selection.json` that
  has Rust or TS;
- GlareDB at `8001afa4`;
- karakeep at `f8ae9866`;
- klin's own `src/` at the baseline commit.

These are not agent changes. They do not count against the cap of 30.

There is no fixed minimum number of groups. No group count proves a BLOCK
gate. A count of n groups gives only an upper bound on the false-block rate.
The result states that bound (section 10).

- Label the union of the groups that the BLOCK-eligible candidates make,
  not only the P2 groups. Start with the P2 groups, then label the groups that
  each more aggressive candidate adds (M-std, N on, E2, O-none, each P3
  class). A candidate's evidence counts only the groups it makes. Report n
  for each candidate and threshold.
- Label as many groups for each language as the budget allows. Aim at about
  150 groups for each candidate that may reach BLOCK. With zero non-copy
  groups, 150 gives a 3/n value of 2% (rule of three).
- Cap the share of one repository at one third of the labeled groups for a
  language. Groups from one repository are correlated, so the real bound is
  weaker than 3/n. Report the count for each repository.
- If a language produces more groups than can be labeled, take a seeded random
  sample inside each repository's cap. Record the seed.

The census measures duplicates that already exist in base trees, not
duplicates that agents add. It is a proxy for the population that matters.
The sample is also capped by repository and correlated. 3/n is therefore a
descriptive number for this sample, not a bound on the future false-block
rate of the product. The result must say this.

Sources for this rule:

- Rule of three: Hanley and Lippman-Hand, "If nothing goes wrong, is
  everything alright?", JAMA 249(13), 1983.
- Clone-detector precision studies usually sample about 400 pairs for a 95%
  level with a ±5% interval. SourcererCC used 390 pairs and three judges
  (https://arxiv.org/pdf/1512.06448). See also
  https://arxiv.org/pdf/1812.05195.

### Tier 4: fresh holdout, only if needed

Use a fresh natural holdout only if tiers 1 to 3 cannot separate two
candidate profiles. Cap: 30 agent changes. Freeze the selection before the
prototype runs on it.

## 8. Measurements

`results.tsv` has one row for each combination of language, profile, variant,
eligibility (E1/E2) and threshold.

Thresholds that can be selected: 60, 80, 100 and 150. Rows at 40 are
diagnostic only. 40 can never be selected, because the census starts at 60.
After the threshold is selected by the rule in section 10, add diagnostic rows
at the selected value plus and minus 10 and 20, so that a brittle cutoff
shows.

Columns:

- `positives_caught`, `positives_required`;
- `hard_negatives_collided` (tier 1);
- `census_groups`, `census_copy`, `census_noncopy`, `census_unlabeled`;
- `units_eligible`, `units_unclear`, `units_excluded`;
- `macro_local_units` (Rust, M-strict), `inline_capture_units` (Rust);
- `unclear_pattern_units` (Rust, for each pattern position);
- `recovered_parse_units`;
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

A **candidate** is one combination of profile, switch `N`, eligibility,
macro variant, owner variant, literal class and threshold. A candidate
**passes** when both of these are true:

1. it catches every required positive whose size is at or above the
   threshold, and at least one copy of each required positive case is at or
   above the threshold;
2. it has zero blocking hard negatives in tier 1, in the census groups it
   makes and in any holdout.

Selection, for each language. The order is total, so no choice remains after
the results:

1. Thresholds: take the lowest threshold in 60, 80, 100, 150 at which some
   candidate passes.
2. At that threshold, take the least aggressive passing candidate. Compare
   candidates in this order of dimensions, with the less aggressive value
   first:
   1. profile: P1, then P2, then P3;
   2. switch `N`: off, then on;
   3. eligibility: E1, then E2;
   4. macros: M-strict, then M-std;
   5. owner: O-type, then O-none;
   6. literal class: none, then `num`, then `str`, then `num+str`.
3. If no candidate passes at any threshold, apply the REVIEW and reject rules
   to the P2, `N` off, E1, M-strict, O-type candidate at 100.

Strength:

- **BLOCK candidate**: the selected candidate passes. The strength states its
  evidence: `0/n` non-copy census groups for that candidate, the number of
  repositories, the sampling scheme, and the 3/n value marked as descriptive.
  Example: "BLOCK candidate: 0 of 152 census groups were non-copy (11
  repositories, capped at one third each, seeded sample). 3/n = 2%, on
  base-tree duplicates; not a bound on future agent findings."
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
- the normalization contract (profile, switch `N`, eligibility, macro
  variant, owner variant, literal class, threshold);
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
  - an item-level macro or glob import in the file, which makes Rust pattern
    units `unclear`;
  - inline format capture instead of a positional argument (Rust);
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
- Do not add a token-neighbor rule for macro arguments. Section 13, check 3
  shows that it fails on closures and struct fields.

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

## 13. Semantic checks, 2026-10-05

Three checks were done before the PR #488 review fixes. Probes ran with rustc
1.98.1 (edition 2021) and tree-sitter 0.27.0 with tree-sitter-rust 0.24.2.

1. **Rust bare identifiers in patterns.**
   - With a lowercase `const x: i32` in scope, `let x = 5`, `fn f(x: i32)`,
     `|x: i32| x` and `for x in 0..3` each fail with E0005 (refutable
     pattern). `match v { x => 0 }` fails with E0004. So `x` is resolved as a
     const pattern, not a binding.
   - With a lowercase unit struct `struct s;` in scope, `let s = s;` and
     `fn f(s: s)` compile. With `const c: () = ();`, `let c = ();` compiles.
     Here the pattern identifier is a path pattern, not a binding.
   - Result: a bare lowercase identifier is not certain to be a binding in any
     pattern position. Section 6 now requires the absence of a same-name item,
     glob import and item-level macro.
2. **Macro hygiene and std macros.**
   - A `macro_rules!` macro that names a caller's local `x` without receiving
     it as a token fails with E0425. Locals are hygienic.
   - `format!("{v}", v = x)` compiles: `v` is a named argument, not a local.
   - `format!("{x}")` compiles: inline capture names a local inside a string
     literal.
   - Result: M-std must treat `name =` as a named argument and must record
     inline capture as a blind spot. The case for M-std rests on the
     documented expression arguments of the listed std macros, not on
     hygiene. The format family is built into the compiler.
3. **tree-sitter-rust macro arguments.** The argument of every macro is a flat
   `token_tree`. Every name is an `identifier` leaf, including closure
   parameters (`vec![|a| a + 1]`), struct fields (`Foo { x: y }`), named
   format arguments (`v = x`) and field access (`a.b`). Only nested brackets
   make nested `token_tree` nodes. Result: a token-neighbor rule gives a wrong
   slot for a closure parameter and erases a field anchor. M-std parses the
   arguments again as expressions instead.
