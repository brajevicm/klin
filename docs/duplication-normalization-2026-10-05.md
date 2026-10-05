# Canonical duplication normalization: design and handover, 2026-10-05

This note belongs to #480 under #478. It is the research design and the
handover for the agent that runs the research. It holds no results yet.

The ratchet model is settled in `docs/duplication-ratchet-lineage-2026-10-05.md`
(#479). This note decides what one canonical token stream means, and it
proposes a stage 0 that decides which duplication unit is worth calibrating.

Status: design reviewed by five adversarial passes. The second to fifth
passes are the PR #488 reviews. Section 2A is a **proposed scope change**,
pending an owner decision on #478, #480 and #483. The scope-independent fixes
of the fifth review are in this design. Its findings on name identity (2 to 4)
wait for that decision. Nothing below has been measured, except the semantic
checks in section 13.

## 1. Scope

In scope:

- a standalone research prototype that makes canonical token streams;
- stage 0 (section 2A): how often agent changes add duplication of four kinds,
  whole functions, exact regions, alpha-renamed regions and near-misses, and a
  cost model for each kind;
- four normalization profiles, P1 to P4, with the variants in section 5, as a
  token-stream contract that every kind of unit uses;
- per-language calibration of Rust and TypeScript/TSX, for the unit that stage
  0 selects;
- a labeled corpus and a frozen decision rule;
- a frozen candidate that #481 can attack.

Out of scope:

- production code in `src/`;
- a product detector for partial or block clones. Stage 0 measures them; #483
  owns their product design;
- agent repair runs;
- a final BLOCK decision;
- the 1M-line performance measurement (#482). Stage 0 builds a cost model on
  the 10k and 300k fixtures only.

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

## 2A. Stage 0: prevalence and cost (proposed)

### Why

Five review rounds made the whole-function fingerprint more careful. None of
them asked how often agents add whole-function copies. Two facts suggest that
the whole-function unit may be the wrong target:

- The reuse failures that reviewers flagged in klin's own research were
  failures to use an existing abstraction, not copies. On GlareDB#3633 the
  reviewer asked for `UnaryInputNumericOperation`. On karakeep#1723 the
  reviewer asked for the existing `useUpdateUserSettings` hook. A
  whole-function detector catches neither.
- Mature clone tools detect duplicated **regions** (token runs, statement
  sequences, blocks or subtrees), not whole functions: SonarQube, PMD CPD,
  jscpd, NiCad, Code Climate and SourcererCC. A copy of the middle of a
  function is their normal case. Whole-function matching is also cheap to
  evade: one added statement changes the fingerprint.

Those tools report duplication. None of them blocks an agent, so their choice
of unit shows what catches copies, not what is precise enough to block.

### Candidates

Every candidate uses the token-stream contract of sections 3, 5 and 6. Units
are eligible functions under E2 (section 4). Test code is excluded.

| Id | Unit | Stream | Match | Intended tier |
|---|---|---|---|---|
| A | whole function | P2 | equal hash | Stop |
| B | maximal token run of at least T tokens inside one function body | P1 | equal hash | Stop |
| B-block | a whole statement block (`if`, loop, `try`, `match` arm or block body) of at least T tokens | P1 | equal hash | Stop |
| C | as B | P2 with prev-encoding (below) | equal hash | Stop |
| D | statement block of at least T tokens | P4 | token-bag overlap of at least 0.8 | `klin check` only |

- A region does not cross a function boundary.
- P2 numbers locals by first occurrence. In a sliding window the first
  occurrence changes with each window, so P2 does not combine with a rolling
  hash. C uses Baker's prev-encoding instead: each local token becomes the
  distance to the previous occurrence of the same local inside the region, and
  0 for the first. The prototype may compute C by brute force for each
  candidate region. Record whether a rolling form was built. If it was not, the
  cost model for C is a lower bound only.
- Region matches in B and C start from winnowed k-gram hashes and extend to
  the maximal matching run. Overlapping windows merge into one region, and
  regions merge into one family before anything is counted.
- D is measured for prevalence only, as the comparator for near-miss copies
  such as GlareDB#3633. It is never a Stop candidate.
- Each candidate is measured at T = 60, 80, 100 and 150.

### Corpus

Agent changes that are already recorded or already defined. None of them is a
fresh agent change, so the cap of 30 does not apply.

1. The #357 pilot agent arm in `docs/phenotype-pilot-2026-10-02/selection.json`:
   10 Rust and 10 TypeScript pull requests, with their `base` and `head`.
2. The Rust and TypeScript pull requests of the pilot human arm in the same
   file, as a contrast: do agents add copies more often than people do?
3. The two #355 natural cases: GlareDB#3633 (`8001afa4` to `44da2223`) and
   karakeep#1723 (`f8ae9866` to `87b39726`).
4. The `natural-agent-rust` and `natural-agent-typescript` populations of
   `docs/phenotype-study-2026-10-04/populations.tsv`: the first 40 eligible
   pull requests per language under that file's rule. Materialize them with
   that rule if no earlier ticket did. Record the selection.

### Measurement

For each change, run every candidate over the head. Report each group or
region family that contains code the change added or modified, and say
whether its other members are old code or also new in the change. This is not
a regression count; #479 owns that question.

Label every reported group with the blind protocol of section 9, with one
extra column: `flagged` is yes when the #357 pilot coded a reviewer comment on
that code as `reuse`.

Report, for each candidate, language, arm and T:

- changes with one or more groups;
- changes with one or more groups labeled `copy`;
- the labeled groups, with the share of each label;
- examples, one per label.

### Cost model

On the 10k and 300k performance fixtures, for each Stop candidate (A, B,
B-block, C):

- index entries and bytes per 1,000 lines;
- cold index build time;
- warm query time per changed file;
- the largest posting list, and the work it causes;
- whether verification needs a read of an unchanged source file;
- the size of klin's current structural cache on the same fixture, as the
  reference for the growth limit.

Extrapolate each number to 1M lines with 20 and with 100 changed files. Show
the extrapolation method. These are models. #482 measures the real 1M numbers
for the candidates that pass.

Region verification and #478's rule of zero unchanged-source reads: a region
match starts as a sampled hash hit, and its exact edges need the tokens of the
other side, which may sit in an unchanged file. Model both ways to avoid that
read:

1. store a chain of positional hashes for base files, so that extension
   compares hashes; this costs index size;
2. at Stop, report only "a copied region of at least T tokens" with
   approximate edges, and compute exact edges at `klin check`.

Common windows (idioms) make very long posting lists. Model a cap: a window
whose posting list exceeds a fixed size is skipped and counted, and a query
that hits the work cap reports INCOMPLETE. Report how many windows the cap
skips and how many labeled `copy` groups it loses.

### Frozen rules for stage 0

Commit these rules before stage 0 measures anything.

Elimination. A candidate stays a Stop candidate only when all of these are
true in the 1M extrapolation:

1. the modeled warm median for 20 changed files is at most 15 ms, and at
   most 50 ms for 100 changed files;
2. the modeled index growth is at most 10% of the structural cache;
3. verification needs no read of an unchanged source file;
4. its work is bounded by a cap, with INCOMPLETE reporting.

A candidate that fails any of these is a `klin check` candidate at most.

Selection. Stage 0 ends with a report to #478. The owner decides the next
step. The report recommends:

- the Stop candidate that remains after elimination and has the most labeled
  `copy` groups in agent changes, for the calibration of sections 7 to 10;
- no Stop duplication gate, if no remaining Stop candidate has any labeled
  `copy` group in agent changes;
- a `klin check` candidate for #483, if D or an eliminated candidate finds
  labeled `copy` groups or flagged reuse failures that the Stop candidates
  miss.

Then sections 7 to 10 calibrate the selected unit. They use the selected
candidate's unit wherever they say "unit", "function" or "group".

## 3. Pipeline

For each source file:

1. Parse with the grammar for the file extension. Use `typescript` for `.ts`,
   `tsx` for `.tsx`. Both map to language `ts`.
2. Find eligible units (section 4).
3. For each unit, collect leaf tokens in source order. Skip comments. For TS,
   apply the statement terminator rule below.
4. Classify each identifier leaf with the binding walker (section 6).
5. Write one canonical stream per profile and variant (section 5).
6. Hash each stream with sha256. Keep the stream text for an equality check.
7. Group units by `(language, profile, variant, hash)`. Confirm that the
   streams in each group are equal.
8. Write one row per group and per threshold.

Use the **text** of each leaf token, never the node kind. The TS and TSX
grammars give different node kinds for some of the same code. The text keeps
TS and TSX as one identity. Fixture `tsx-same-as-ts` checks this.

TS statement terminators. In TS, a line break can change the meaning through
automatic semicolon insertion (ASI). tree-sitter inserts the automatic
semicolon as a hidden token, so leaf text alone cannot show it. Section 13,
check 4 shows four pairs of functions with different behavior and the same
leaf text. The trees of these pairs differ. Rule:

1. A **terminated kind** is a node kind whose rule in the tree-sitter
   JavaScript or TypeScript `grammar.js` ends with `$._semicolon` (an explicit
   `;` or an automatic one). Examples: `expression_statement`,
   `return_statement`, `break_statement`, `continue_statement`,
   `throw_statement`, `lexical_declaration`, `variable_declaration`,
   `public_field_definition`. List the full set from `grammar.js` of the
   pinned versions, and commit the list with the prototype.
2. Inside a node of a terminated kind, drop a final `;` leaf. Then emit one
   `;` token at the end of the node. Do this for explicit and automatic
   semicolons alike.
3. Keep every other `;` leaf, for example the separators in `for (;;)`.

So `return⏎value;` gives `return ; value ;`, and `return value;` gives
`return value ;`. Code with and without semicolons gives the same stream when
ASI makes them equal. These node kinds are the same in the TS and TSX
grammars, so the rule keeps one identity.

TS block comments with a line break. Under ECMAScript, a block comment that
contains a line break counts as a line terminator for ASI. tree-sitter 0.23.2
does not apply this: it parses `return /*⏎*/ value;` as `return value;`
(section 13, check 4). The tree is wrong, so no stream rule can repair it. A
TS unit is `unsafe` (section 6) when it contains a block comment with a line
break and code on the same line before or after the comment. Report the
count.

Rust has no ASI. Rust line breaks never change the meaning, so Rust needs no
terminator rule.

The size of a unit is the number of non-comment leaf tokens in the source,
with punctuation included, counted before any transform. Shorthand expansion
(section 5) adds tokens to a stream, so the stream length is not the size.
The size is equal for every profile and variant of one unit. A collision group
qualifies for a threshold when its smallest unit is at or above it. The size
is not comparable between Rust and TS. Each language gets its own threshold.

A unit whose subtree contains an `ERROR` node or a missing node is `unsafe`
(section 6). Report the count for each language.

Parser-risk suite. A wrong tree that tree-sitter accepts without an error node
matters only when it can make two different units give the same stream. The
stream is leaf text, terminators and identifier classes. Leaf text does not
depend on the tree, so only two things can go wrong: terminator emission and
identifier classification. The prototype keeps a fixed list of known silent
misparse classes for the pinned grammars, with one fixture each, and a rule
for each class:

| Class | Pinned-grammar result (section 13) | Rule |
|---|---|---|
| ASI pairs (`return`, postfix `++`/`--`, `break`/`continue` label, `async`) | tree differs, correct | terminator rule (above) |
| block comment with a line break next to code | tree wrong | `unsafe` |
| `await` as an identifier outside an `async` function | `ERROR` node | `unsafe` |
| `yield` as an identifier outside a generator | silent `yield_expression` | `unsafe` |
| regex versus division | correct | none |
| `let` as an identifier | correct | none |
| `f<T>(x)` versus `(f < T) > (x)` | correct | none |

The research agent adds two more probes before measurement: an arrow
function inside a conditional expression, and a type assertion `<T>x` in a
`.ts` file. The list and its rules freeze with the protocol (section 10).
If a new class appears after measurement starts, write a new frozen parser
contract and run every affected tier again. Do not add the rule in place.
This list cannot prove that no other misparse exists. The result must say
so.

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

Measure E1 and E2 separately. E1 is the candidate eligibility. E2 is a
comparator (section 10).

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
       macro keeps M-strict text (state `kept`).
    2. In the format family, a top-level argument of the form `name = expr`
       is a named format argument. `name` is an anchor. `expr` is walked.
    3. The format string keeps its text in both variants. An inline capture
       such as `"{x}"` is not normalized, so a renamed copy that uses inline
       capture does not match. Record this as a blind spot, with a count.
    4. Every other macro keeps M-strict text.

  M-std is REVIEW only and a comparator (section 10). A local
  `macro_rules! vec` or an import can shadow a std macro, and
  `#[macro_use] mod m;` in a parent module makes a macro of another file
  visible. A check inside one file cannot prove that `vec!` is the std macro.
  Hard negative `shadowed-std-macro` shows the risk. Record how many units
  contain a local name inside a macro under M-strict.
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

  P3 is a comparator (section 10). Report the groups each class adds and
  their labels. Hard negative `validation-constants` will likely show the
  risk of literal normalization. #478 decides whether any class is admitted
  later.

## 6. Binding walker

This walker covers one function at a time and uses only lexical scope. It
is not a name resolver. If correct output needs compiler-scale resolution,
record that as evidence against P2 and fall back toward P1 (see #480).

Every unit has exactly one state:

| State | Meaning | Can block |
|---|---|---|
| `clear` | the walker classified every identifier | yes |
| `kept` | one or more identifiers kept their original text because the walker could not classify them safely | yes |
| `unsafe` | the tree or the terminators may be wrong | no |

Keeping the original text never makes two different units equal, so a `kept`
unit is safe to block. The cost of `kept` is recall: a renamed copy of it
does not match. `unsafe` units are fingerprinted and reported, but they are
never part of a blocking group.

Because `kept` costs recall, a candidate cannot buy precision by keeping text.
Its cost shows up as lower natural recall (section 10).

Rule for unclear identifiers: **keep the original text**. Exclude a unit only
when keeping the text would still give a wrong stream. Exclusion is a way for
an agent to avoid the gate (section 11), so use it as little as possible.
Record each exclusion with its reason.

Every identifier leaf gets exactly one class:

| Class | Output in P2/P3 | Examples |
|---|---|---|
| binding | new slot | `let x`, parameter `a`, `catch (e)` |
| local-ref | slot of the binding found in scope | `x + 1` |
| own-name | `SELF` with switch `N` on, else original text | bare recursive call of a free function |
| anchor | original text | field, member, type, path, macro name, global |
| import | provenance (see below) | a name bound by a `use` or `import` in the file |
| kept | original text | identifier inside a Rust macro (M-strict), a pattern identifier the walker cannot prove to be a binding |

Rust rules:

- Bindings: parameter patterns, `let` patterns, `for` patterns, `match` arm
  patterns, `if let` and `while let`, and closure parameters.
- Each block, closure, match arm and `if let` body opens a new scope.
- A new `let` shadows the old binding with a new slot.
- Before the pattern rule below: an item declared in a block is in scope in
  the whole block, also before its declaration. A macro invocation in
  statement position can expand to such an item, for example a `const`, and
  a macro invoked later in the block still changes an earlier pattern
  (section 13, check 5). tree-sitter cannot tell an item macro from an
  expression macro in statement position. So a bare lowercase pattern
  identifier keeps its text (state `kept`) when the unit's block or any
  enclosing block contains, anywhere, a macro invocation in statement
  position. This includes `println!(…);`.
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
     item with that name;
  4. no item in the file has an attribute outside this inert list: `cfg`,
     `allow`, `warn`, `deny`, `forbid`, `expect`, `doc`, `inline`, `cold`,
     `must_use`, `deprecated`, `track_caller`, `repr`, `non_exhaustive`,
     `test`, and `derive` of only these std traits: `Debug`, `Clone`,
     `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Default`.
     Any other attribute may be a proc macro that adds names. Two limits:
     - `cfg_attr(cond, a, b, …)` is inert only when every attribute it
       carries is inert, checked recursively. It can expand to a proc macro.
     - the std derive names count as inert only when the file imports none
       of those names and has no glob import, because an import can make
       `Debug` resolve to a user derive.

  Otherwise the identifier keeps its text (state `kept`). Report the `kept`
  rate for each pattern position. A high rate is evidence against P2 for
  Rust, not a reason to relax the rule. `ref`, `mut` and `x @ pat` follow the
  same rule for `x`.
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
- Strict code: a file with an `import` or `export` statement (a module), the
  body of a class, and code under a `"use strict"` directive. TS output is
  usually a module, but the walker must check this for each file.
- Before the walk, hoist the `var` bindings to the top of their function
  scope. A `let`, `const` or `class` binding belongs to its block.
- A function declaration directly in a function body is hoisted to the top
  of that body.
- A function declaration inside a block:
  - in strict code, it belongs to that block, like `let`;
  - in other code, Annex B rules apply. The walker does not model them: every
    identifier in the unit keeps its text (state `kept`).
- Parameters: when any parameter has an initializer or a destructuring
  pattern with an initializer, the parameters get their own scope between the
  outer scope and the body. An initializer sees only the parameters before it
  and the outer scope, never a `var` or function declaration of the body. A
  body `var` with the same name as a parameter is a new binding in the body
  scope.
- Each block, arrow and function expression opens a new scope.
- Keep these as anchors: `this`, `super`, `property_identifier`,
  `type_identifier`, the names of generic type parameters, JSX element and
  attribute names, import names, labels, and identifiers that do not resolve
  in scope.
- `typeof x` inside a type position: keep the text.
- `eval(`, `with`, and `arguments`: keep every token in the unit as P1 text
  (state `kept`). Do not exclude the unit.

Imports, both languages. A local import alias hides which API a name means:
`import { parse as run } from "parser-a"` and
`import { execute as run } from "parser-b"` give the same text `run`. So an
identifier that resolves to an import binding in the file is an `import`
class, and it emits its **provenance**:

- TS named import: `"<specifier>"#<imported name>`. Default import:
  `"<specifier>"#default`. Namespace import `* as ns`: `"<specifier>"#*`.
- TS relative specifier (`./`, `../`): resolve it against the file's
  directory to a repository-relative path, by path arithmetic only. Do not
  add an extension or read `tsconfig.json`. Any other specifier keeps its
  text.
- Rust: the full written `use` path with the alias replaced by the original
  last segment. `use parser_a::parse as run;` makes `run` emit
  `parser_a::parse`. `crate::`, `super::` and `self::` paths keep their
  written text. A name from a glob import cannot be resolved: it keeps its
  text.
- P1 emits the alias and the provenance, for example
  `run@"parser-a"#parse`, so P1 stays exact. P2 and P3 emit only the
  provenance.
- A name that resolves to an item declared in the same file keeps its
  spelling. Probe `same-name-local-helper` measures the risk (section 7).

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

Required hard negatives (must not collide under a candidate, P1 or P2):

- `same-skeleton-different-api` (both functions call the API through the
  same local alias, `run`, imported from different modules);
- `crud-wrappers`;
- `serializers-different-schema`;
- `validation-constants`;
- `react-boilerplate` (TSX);
- `trait-required-methods` (Rust) and `interface-required-methods` (TS);
- `error-adapters`;
- `generated-protocol`;
- `table-dispatch`;
- `test-helpers`;
- `owner-only-differs` (the same body in two `impl` blocks or classes; result
  depends on O-none / O-type);
- `shadowed-std-macro` (Rust: a file with a local `macro_rules! vec` whose
  arguments are not expressions);
- ASI pairs (TS), each with the same leaf text and different behavior:
  `asi-return`, `asi-postfix`, `asi-break-label`, `asi-async`, and
  `asi-block-comment` (must be `unsafe`, see section 3);
- one fixture for each class of the parser-risk suite (section 3).

Anchor tests (#480 AC on API/member/type preservation). Each pair differs in
exactly one anchor. It must not collide under P1 to P3 and must collide under
P4:

- `anchor-method`, `anchor-field`, `anchor-type`, `anchor-enum-variant`,
  `anchor-imported-fn`, `anchor-macro` (Rust), `anchor-jsx-element` (TSX).

Measured probes. They are reported, but they are neither required positives
nor required hard negatives, so they cannot decide a candidate by
construction. The census labels decide what they mean:

- `same-name-local-helper` (two files, each with its own `helper` with a
  different body, and the same caller; spelling-only names collide here by
  design);
- `glaredb-executor` (the same executor scaffold with a different member or
  API, modeled on GlareDB#3633; the reviewer wanted this code changed to use
  an existing abstraction, so it is a reuse failure, not a safe negative).

Optional recall probes. They are reported but are not required, so no
variant is forced by them:

- `local-rename-macro` (Rust: renamed local passed as an argument to
  `format!` and `assert_eq!`; expected to match only under the M-std
  comparator);
- `local-rename-inline-capture` (Rust: renamed local in `"{x}"`; expected to
  miss in every variant);
- `function-rename-recursive` (expected to match only with switch `N` on);
- `method-rename-recursive` (expected to miss in every variant).

Binding construct tests, both languages where the construct exists:

- `shadowing`, `destructuring`, `closure-capture`, `match-pattern-binding`
  (Rust), `uppercase-pattern-const` (Rust), `var-hoisting` (TS),
  `catch-binding` (TS), `bound-arrow`, `anonymous-callback`,
  `member-vs-local-same-name` (`x.x`), `kept-eval` (TS),
  `lowercase-unit-struct-pattern` (Rust: `struct s; let s = s;` must be
  `kept`), `macro-closure-arg` (Rust: `vec![|a| a + 1]`),
  `macro-struct-field` (Rust: `vec![Foo { x: y }]`), `format-named-arg`
  (Rust: `format!("{v}", v = x)`), `recovered-parse` (both: a unit with an
  `ERROR` node, must be `unsafe`), `statement-macro-before-pattern` and
  `statement-macro-after-pattern` (Rust: a statement macro before or after a
  `let` pattern in the same block keeps the pattern text), `proc-macro-attribute`
  (Rust: an item with a non-inert attribute keeps lowercase pattern text),
  `default-param-body-var`,
  `default-param-body-function`, `block-function-strict` and
  `block-function-module` (TS), `block-function-sloppy` (TS: must be
  `kept`), `semicolon-style` (TS: the same code with and without
  semicolons must match), `import-alias-same-provenance` (both: two aliases
  of the same import must match under P2).

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
gate. The result states `0/n` and 3/n as descriptive calibration-sample
evidence (section 10).

- Label the groups of the candidates first (section 10). Then label the
  groups that each comparator adds (M-std, N on, E2, O-none, each P3 class),
  so that the result can report their precision. Report n for each candidate,
  comparator and threshold.
- Label as many groups for each language as the budget allows. Aim at about
  150 candidate groups. With zero non-copy groups, 150 gives a 3/n value of
  2% (rule of three).
- Cap the share of one repository at one third of the labeled groups for a
  language. Groups from one repository are correlated, so the real bound is
  weaker than 3/n. Report the count for each repository.
- If a language produces more groups than can be labeled, take one seeded
  random sample from the groups of size 60 or more, inside each repository's
  cap. Record the seed. The evidence for a higher threshold is the subset of
  this same sample, so that sampling noise cannot change which threshold
  passes. Report n for each threshold. When the subset for a threshold has
  fewer than 30 labeled groups, its strength wording says so. The frozen sample is
  the evidence population. The cap gives repositories unequal inclusion
  probabilities, so the sample is not a simple random sample of all groups.
  `0/n` and 3/n describe the calibration sample only. The strength wording
  must state the sampled fraction, for example `0/150 sampled from 2000`.

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
- `units_eligible`, `units_clear`, `units_kept`, `units_unsafe`,
  `units_excluded`, and `clear_pct`;
- `units_in_groups` (clear or kept units that are in a collision group);
- `natural_copy_groups`, `natural_caught`, `natural_recall` (section 10);
- `macro_local_units` (Rust, M-strict), `inline_capture_units` (Rust);
- `kept_pattern_units` (Rust, for each pattern position);
- `recovered_parse_units`;
- `walker_us_p50`, `walker_us_p95`, for each unit. This is a rough cost for
  #482, not a gate.

## 9. Labels

Write `labels/protocol.md` before the first census run, and commit it before
any label.

Label each group with exactly one label. Look at every production member of
the group first:

| Label | Meaning |
|---|---|
| `copy` | meaningful copied implementation; a reviewer would ask for reuse or extraction |
| `boilerplate` | framework or language ceremony |
| `required-shape` | a trait, interface or protocol requires the shape |
| `generated` | generated or protocol-style code |
| `test` | test code or a test helper |
| `distinct` | same shape, different responsibility |
| `mixed` | some members are one copied family, and one or more members are independent of it |

`copy` means that every production member belongs to one copied family. If
any member is independent, the label is `mixed`, not `copy`.

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

Audit packet. The result note includes a file `labels/audit.tsv` with these
groups, for a human or an independent second judge:

- every group with a label other than `copy`;
- every `mixed` group and every group with more than three members;
- every group that a comparator adds over the selected candidate;
- a seeded random sample of 20 `copy` groups for each language.

#480 does not do the audit. The human audit happens after #480 freezes its
candidate and before #481 starts, because #481 attacks the frozen candidate
and a rejected label set would waste that work. #482 measures the
representation and the index, which do not depend on the labels, so #482
may run in parallel. Until the audit, the strength wording says
"agent-labeled, unaudited".

## 10. Frozen decision rule

Commit this rule before measurement. Do not change it after you see results.

A **blocking hard negative** is a collision group at or above the threshold,
in production scope, with any label other than `copy` (`mixed` included).

Candidates and comparators. Every variant that only adds collisions to a less
aggressive value cannot win this rule, because no required positive needs it.
So these are **comparators**, never candidates: switch `N` on, E2, M-std,
O-none, every P3 literal class, P4, and `R-lint` (below). The result reports,
for each comparator, the groups it adds over the selected candidate and their
labels, and its natural recall.

#478 may later promote a comparator. A promoted comparator is a new frozen
candidate. It must go through #481 again, and through #482 when it changes
the representation, the index or the work done.

`R-lint` (Rust): treat a bare lowercase pattern identifier as a binding when
the conditions 1 to 4 of section 6 fail only because of macros or
attributes, and no `#![allow(non_upper_case_globals)]` or
`#![allow(non_camel_case_types)]` (or an item-level `allow` of these) is in
the file or the crate root. It relies on the warn-by-default naming lints.
It measures how much recall a naming-convention assumption would add.
Adopting it is a product decision for #478 and needs an ADR.

P1 is the **baseline**. It cannot pass, because `local-rename` is a required
positive and P1 keeps local names. It stays in every table as the reference
for natural recall.

A **candidate** is P2, with `N` off, E1, M-strict and O-type, at one
threshold. A candidate **passes** when both of these are true:

1. it catches every required positive whose size is at or above the
   threshold, and at least one copy of each required positive case is at or
   above the threshold;
2. it has zero blocking hard negatives in tier 1, in the labeled census sample
   of the groups it makes, and in any holdout.

Selection, for each language. The order is total, so no choice remains after
the results:

1. Take the lowest threshold in 60, 80, 100, 150 at which the candidate
   passes.
2. If it passes at no threshold, apply the REVIEW and reject rules to it at
   100.

Strength:

Natural recall. A **natural copy group** is a census group labeled `copy`,
from the union of the groups of every candidate and comparator, P4
included. A candidate **catches** a natural copy group when it puts two or
more of its members into one of its own groups. `natural_recall` is the
caught fraction. Report it for every candidate and comparator.

- **BLOCK candidate**: the selected candidate passes, and, if it is P2, its
  natural recall is greater than the natural recall of P1 at the same
  threshold. P2 exists to add rename recall. A P2 that adds none, for example
  because most units are `kept`, has not earned BLOCK and gets REVIEW. The
  strength states its evidence: `0/n` non-copy census groups for that
  candidate, the number of repositories, the sampling scheme, the 3/n value
  marked as descriptive, `clear_pct`, `units_kept`, `units_unsafe`, and the
  natural recall of the candidate and of P1.
  Example: "BLOCK candidate: 0 of 152 labeled census groups were non-copy,
  sampled from 2000 (11 repositories, capped at one third each, seed 7).
  3/n = 2%, on base-tree duplicates; not a bound on future agent findings.
  Agent-labeled, unaudited."
- **REVIEW**: the census has more `copy` groups than other groups.
- **reject**: none of the above.

Rust and TS are judged independently. They may get different profiles,
variants and thresholds.

## 11. Output for #481

The result note must contain one frozen candidate for each language. Each
candidate states:

- eligible units;
- the normalization contract (profile and threshold; the fixed values of
  the other dimensions);
- the comparator results: the groups each comparator adds, with labels;
- the semantic anchors preserved;
- the minimum size;
- unsupported behavior, and the `clear`, `kept` and `unsafe` counts;
- the natural recall of the candidate, of P1 and of each comparator;
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
  - an item-level macro, a glob import or a non-inert attribute in the file,
    which keeps Rust pattern text (`kept`);
  - inline format capture instead of a positional argument (Rust);
  - a statement macro such as `println!(…);` anywhere in the block of a
    pattern (Rust), which keeps the pattern text (`kept`);
  - a block comment with a line break next to code (TS), which makes the unit
    `unsafe`;
  - an import alias with a glob or unresolved provenance;
  - adding `arguments` or `eval` (TS).

  Measure each evasion on the `local-rename` fixture.

## 12. Handover

### Start state

- This note is on branch `issue-480-normalization-research`, PR #488
  (`Refs #480`). The PR went through four review rounds. The fourth review's
  findings 2 to 4 (name identity) wait for the scope decision in section 2A.
  Read all PR comments before step 2.
- Section 2A is a proposed scope change. Do not start until the owner has
  decided it on #478. If the owner rejects stage 0, skip steps 2 to 4.
- Start the research on a new branch from `main` after PR #488 merges. If
  PR #488 is not merged, ask the user which branch to use.
- Code baseline for the census of klin's own `src/`: `b61ac917` on `main`.
- Only the probes in section 13 ran. Nothing in the corpus is measured. No
  comment on #478 or #480 has been posted.

### Steps

1. Read #480, #478, #483, `docs/duplication-ratchet-lineage-2026-10-05.md`
   and this note.

Stage 0:

2. Write `labels/protocol.md`, the frozen rules of section 2A, and the
   decision rule in section 10. Commit them alone, before any measurement.
3. Build the prototype: parse, eligibility, token streams (section 3), the
   walker (section 6), the profiles (section 5), then the candidates A, B,
   B-block, C and D (section 2A). Add walker unit tests over the construct
   fixtures. Run the TS `locals.scm` cross-check and explain every
   difference.
4. Run stage 0: prevalence over the corpus of section 2A, blind labeling, and
   the cost model on the 10k and 300k fixtures. Write the stage 0 report
   below a `## Stage 0 results` heading. Draft the #478 comment with the
   recommendation, run the `humanizer` skill on it, and **ask the user before
   posting it**. Stop until the owner decides.

Calibration of the selected unit:

5. Write the tier-1 fixtures and `expected.tsv` for the selected unit.
   Commit.
6. Make every tier-1 row in `expected.tsv` pass, or record why a row fails.
   Do not change an expected value to make a row pass without a written
   reason.
7. Run tier 2. Export trees with `git archive` into a temp directory. Do not
   commit third-party source trees. Commit only the derived TSV rows and short
   excerpts.
8. Run the tier-3 census, then blind labeling.
9. Fill `results.tsv`. Select thresholds. Add the rows around each selected
   threshold.
10. Use tier 4 only if section 7 allows it.
11. Write the result section and the frozen candidates (section 11) into this
    note, below a `## Results` heading. Write `labels/audit.tsv`.
12. Run `/code-review` on the branch.
13. Open a PR with `Closes #480`. Run the `humanizer` skill on the PR body
    first.
14. Draft the #478 update comment with the winning candidates. Run the
    `humanizer` skill on it. **Ask the user before posting it.**
15. Tell the user that the human audit of `labels/audit.tsv` is due before
    #481 starts.

### Rules

- Never commit to `main`.
- Do not edit `klin.json`, the hooks, or the `accepted` list.
- Do not edit `src/`. If the pre-commit gate blocks a docs-only commit,
  stop and ask the user.
- Do not run the full Rust suite or benchmarks. The user runs them. The
  stage 0 cost model uses the 10k and 300k fixtures only, never 1M.
- Keep CI runs to a minimum (CI budget).
- Never change the decision rule or the labels after seeing results. If the
  rule is wrong, record the problem and ask the user.
- Never guess a local binding for a candidate fingerprint. Keep the text.
- Do not add a token-neighbor rule for macro arguments. Section 13, check 3
  shows that it fails on closures and struct fields.
- Do not build a TS stream from leaf text alone. Section 13, check 4 shows
  that ASI makes this unsound.

### Known risks

- P2 may need resolution beyond lexical scope in Rust (macros, `use` inside a
  function body). If so, record it as evidence against P2 for Rust and fall
  back toward P1. Do not grow the walker into a resolver.
- The census may be dominated by `required-shape` groups in Rust trait
  implementations. If so, the O-type variant and the owner policy decide the
  result. Report both.
- dify#1422 may be out of language scope.
- Stage 0 may find few labeled agent copies of any kind. Then the
  recommendation is no Stop duplication gate. That is a valid result.
- The region index (B, C) may break the 10% growth limit, or region
  verification may need reads of unchanged files. Then regions are a
  `klin check` candidate at most.
- The labels are agent-drafted. The audit packet (section 9) is for the human
  audit before #481.
- The Rust macro and attribute rules may make most Rust units with patterns
  `kept`. Then P2 may not beat P1 on natural recall, and Rust gets REVIEW.
  That is a valid result. Report the rate and `R-lint`. Do not relax the
  rule.

## 13. Semantic checks, 2026-10-05

Checks 1 to 3 were done before the fixes for the first PR #488 review. Check
4 was done before the fixes for the second review, and check 5 before the
fixes for the third. Probes ran with rustc
1.98.1 (edition 2021) and tree-sitter 0.27.0 with tree-sitter-rust 0.24.2 and
tree-sitter-typescript 0.23.2.

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
4. **TS automatic semicolon insertion.** Probe after the second PR #488
   review, with tree-sitter-typescript 0.23.2, `typescript` and `tsx`
   grammars, same results in both. Leaf text with comments dropped:

   | Pair | Leaf text | Tree |
   |---|---|---|
   | `return⏎value;` / `return value;` | same | different: `return_statement` + `expression_statement` |
   | `x⏎++y` / `x++⏎y` | same | different |
   | `break⏎l;` / `break l;` | same | different: label is a new statement |
   | `async⏎function f(){}` / `async function f(){}` | same | different: `async` is an expression statement |
   | `return /*⏎*/ value;` / `return value;` | same | **same**: tree-sitter ignores the line break in the comment |

   No tree has an `ERROR` node. tree-sitter does not expose the automatic
   semicolon as a leaf. Result: section 3 adds the terminator rule for the
   first four pairs and makes the fifth case `unsafe`.
5. **Third review probes.** rustc 1.98.1 and tree-sitter-typescript 0.23.2.
   - Rust: with `macro_rules! m { () => { const x: i32 = 1; } }`, the body
     `let r = match v { x => 0 }; m!(); r` fails with E0004. A macro invoked
     **after** the pattern in the same block makes `x` a const pattern. A
     later `const x` in an enclosing scope does the same. Result: section 6
     looks at every statement macro in the block, before and after.
   - TS: `var await = 1; return await;` outside an `async` function gives an
     `ERROR` node. `var yield = 1; return yield;` outside a generator parses
     silently as a `yield_expression`. Regex versus division,
     `let` as an identifier and `f<T>(x)` parse correctly. Result: the
     parser-risk suite in section 3.
