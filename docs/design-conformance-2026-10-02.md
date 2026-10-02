# Design and reuse conformance beyond exact duplication, 2026-10-02

This note belongs to #355. It asks one question:

> Can klin identify new code that bypasses a structural or reuse relationship
> that the repository already established, with enough precision and
> explanation to become useful agent feedback?

The target is conformance to the repository's own relations. It is not
generic design advice. The note is research only. It adds no gate, no SOLID
score, no semantic index and no production dependency. It changes no shipped
behavior and edits no SPEC semantics. A disposition here does not authorize an
implementation. That is a separate ticket after a person reviews this note.

#48 owns exact and canonical function duplication. This note does not measure
that. Where a prototype row matches #48's scope, the row says so and the note
does not judge it.

## Terms

This note uses the words of #355, #361, #362 and #364. They are not terms of
`CONTEXT.md`:

- A **candidate** is one predicate this research measures. It is not a gate.
- A **relation** is one fact about two named things of a tree: a type
  implements a trait, a list registers a type, a function is the only caller
  of a call path, a component imports another component.
- A **family** is a trait or interface that types in two or more sibling files
  of one directory implement. The **members** are those types.
- A **site** is one place a candidate matches. A **finding** is a site whose
  status is `finding`. A site can also read `unknown` (the relation needs a
  fact that syntax does not show), `policy` (a person's configuration owns the
  relation), `excluded` (an exclusion tag covers it) or `canonical` (#48 owns
  it).
- A **plant** is planted code that a candidate must find. A **hard negative**
  is planted code that a candidate must not find. An **attack** is a planted
  change that keeps the defect and changes its form, as #355 lists them.
- **`relations`** is the throwaway prototype that measures every candidate,
  `docs/design-conformance-2026-10-02/prototype/`. It is not klin code.

## 1. Hypotheses

#355 summarizes a body of research: code models find it hard to keep design
patterns and architectural intent, and generated code shows redundancy,
non-reuse and design smells. This note did not read those papers again and
cites none of them. It takes the claims from the ticket as stated.

Hypotheses that follow from that literature:

- **P1.** A pattern classifier (GoF, SOLID) applied to a change finds the
  violations that matter. #355 calls this the adversarial baseline.
- **P2.** Generated code recreates operations that the repository already
  has, with lexical differences that defeat exact matching.
- **P3.** Generated code adds indirection with no purpose: wrappers,
  factories and adapters that only forward.
- **P4.** Generated code adds dependency shapes that break the repository's
  structure.

Hypotheses specific to klin:

- **K1.** A repository states its own design in relations that syntax shows:
  `impl Trait for Type`, `class X implements I`, a registration list, a
  function that alone calls a path. A change that bypasses such a relation is
  a repository fact, not a doctrine.
- **K2.** The relations that K1 needs are close to the facts that klin's
  structural extraction (`FileFacts`) and module graph already hold, so a
  candidate needs a named extension, not a second parser or a semantic index.
- **K3.** Most of these relations are ambiguous on their own. A family can
  have an intended exception. So most candidates can be REVIEW evidence at
  most, and only a relation that the repository states as policy can block.
- **K4.** A message that names the relation ("3 existing providers implement
  PaymentProvider, AdyenProvider does not") gets a better repair from an agent
  than a pattern name, and lets a person keep an intended exception.

## 2. Relation taxonomy

| # | Relation | Syntax that states it | Candidate |
| --- | --- | --- | --- |
| R1 | implementation family | `impl F for T` (Rust), `class T implements F` or `extends F` (TS), in two or more sibling files | `family-bypass` |
| R2 | registration | one expression that names two or more members of a family: an array, a `vec!`, an object literal, a `match` or a `switch` | `registration-bypass` |
| R3 | sole caller | every base call of one call path sits in one short exported function, and other code calls that function | `wrapper-bypass` |
| R4 | similar operation | a new function whose syntax shape or name tokens are close to an existing function, and #48's canonical identity is not equal | `near-clone` |
| R5 | forwarding | a new function whose body only passes its parameters to one call | `delegation-only` |
| R6 | component direction | an import from one top-level component to another, in the module graph | `component-cycle` |
| R7 | stated contract | a person's machine-readable rule: klin `layering`, an ESLint restriction, a Clippy `disallowed-*` list, a dependency-cruiser rule | survey only |

R1, R2 and R3 are the "middle layer" that #355 asks about: they are not exact
predicates such as a cycle, and they are not generic design review. R4 is the
"repository reuse candidate" of #355, kept apart from #48. R5 is "new
delegation-only indirection". R6 is "component dependency shape", only where
the module graph proves the edge. R7 is "explicit repository design
contracts".

## Rules, written before any run on the sample

The commit that adds this section holds the prototype, the planted corpus and
its expected rows, the replay scripts and these rules. It holds no result of
the ordinary sample, the agent stratum, the natural cases, the agent repair
runs or the baseline.

### Candidates

| Candidate | Predicate, in short |
| --- | --- |
| `family-bypass` | A new type in a directory with a family is sibling-like and does not implement the family. |
| `registration-bypass` | A new member of a family appears in no registration of that family and other code uses it directly, or a new branch on a string literal sits in a function that reads the family's registration. |
| `wrapper-bypass` | A new call of a call path outside the one wrapper that the base calls it from. |
| `near-clone` | A new function of 50 or more syntax nodes whose shingle similarity (`structural`) or name-token similarity (`lexical`) with an existing function is 0.7 or more. |
| `delegation-only` | A new function whose body is one call with the function's own parameters, in order, and nothing else. |
| `component-cycle` | A new edge between top-level components that closes a cycle the base did not have. |

### `family-bypass` in detail

The unit is a directory `D`. The **sibling files** of `D` are the Rust and
TypeScript files directly in `D` that are not test or generated files and are
not a module root (`mod.rs`, `lib.rs`, `main.rs`, `index.ts`, `index.tsx`,
`index.mts`).

A **family** `F` of `D` is a trait or interface that types of two or more
base sibling files implement:

- Rust: `impl F for T`, with `F` read as its last path segment and without
  generics. A `#[derive(F)]` counts toward conformance, never toward a family.
  These standard traits never make a family: `Clone`, `Copy`, `Debug`,
  `Default`, `Display`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`,
  `Drop`, `From`, `Into`, `TryFrom`, `AsRef`, `AsMut`, `Deref`, `DerefMut`,
  `Borrow`, `Iterator`, `IntoIterator`, `Error`, `FromStr`, `Send`, `Sync`,
  `Serialize`, `Deserialize`, `Future`, `Fn`, `FnMut`, `FnOnce`, `Add`, `Sub`,
  `Mul`, `Div`, `Neg`, `Not`, `Index`, `IndexMut`, `Extend`, `FromIterator`,
  `Write`, `Read`.
- TypeScript: `class T implements F` and `class T extends F`.

A new type `T` (a Rust `struct`, `enum` or `union`, or a TypeScript class)
whose name no base type in `D` has is **sibling-like** to `F` for any of
three reasons:

1. `shape`: `T`'s file is new, and it declares a top-level name of a form
   that two or more base sibling files declare, more than half of which
   implement `F`. The form of a name is the name in lower case without
   separators, with the file stem replaced by `*`: `FUNCTION_SET_COS` in
   `cos.rs` has the form `functionset*`.
2. `suffix`: the last word of `T`'s name is the last word of more than half of
   `F`'s members, and of two or more: `AdyenProvider` and `*Provider`.
3. `methods`: `T` defines every method that all of `F`'s implementations
   define, and that set is not empty.

`T` **conforms** when it implements `F`, or when the reason is `shape` and some
type of `T`'s file implements `F`. A type whose own declaration names `F` (a
field or constructor parameter typed `F`) uses the family and is not
sibling-like. A finding is a sibling-like type that does not conform. The
message names the reasons, the share of sibling files that implement `F`, and
any other family of `D` that `T` implements instead.

The status is `unknown` in place of `finding` when the relation needs a fact
that syntax does not show:

- the TypeScript class has a decorator, so a framework may register it.
- the Rust file invokes a macro at the top level, which may generate the
  `impl`.

### `registration-bypass` in detail

A **registration** of `F` is one expression that names two or more members of
`F` as identifiers that start with an upper-case letter: a Rust array, tuple,
`match` or macro invocation, or a TypeScript array, object or `switch`. Its
**binding** is the name of the `let`, `const` or `static` that holds it, or
the name of the function that holds it.

Two forms are findings:

1. `unregistered`: a type that implements `F` in the after tree and not in
   the base, that no registration of `F` names, while the base has a
   registration of `F`, and that a call or `new` outside its own file names.
2. `branch`: a new conditional on a string literal (a Rust `if` with `==` or
   a `match` arm with a string pattern, a TypeScript `if`, ternary or `switch`
   case) in a function that names the binding of a base registration of `F`,
   outside every registration.

The status is `unknown` when a file in the directory of the member, the
branch or the registration discovers code at run time: TypeScript
`import.meta.glob`, `require.context`, `import()` or `require()` with a
computed argument, Rust `inventory::submit!`, `register!`,
`#[distributed_slice]` or `#[ctor]`.

### `wrapper-bypass` in detail

A call path is **specific** when its text, after the prototype removes white
space and type arguments, has three or more segments, starts with no
`this`, `self`, `super` or `Self`, and holds no call or index of its own:
`transport.http.send` and `crate::http::transport::send` are specific,
`send` and `api.get(x).then` are not.

For a specific path `X`, the base **wrapper** `W` is a function such that:

- every base call of `X` outside test and generated files sits in `W`.
- `W` is exported (`pub` in Rust, `export` in TypeScript), and its text is 15
  lines or fewer.
- some base call outside `W` names `W` by its last segment.

A finding is a call of `X` in the after tree, outside `W` and outside test
and generated files, whose pair of enclosing function and path the base does
not hold. The message names `W`, its location and its count of call sites.

### `near-clone` in detail

Each function body gives three facts:

- `size`: the count of named syntax nodes.
- `shingles`: the set of hashes of every five consecutive named-node kinds in
  the body, in pre-order.
- `canonical`: a hash of the body's leaf tokens with every identifier read as
  one token and every literal as another.

A function is new when its file, owner and name are absent from the base. A
new function of size 50 or more is compared with every function of size 50
or more that the base and the after tree both hold, outside test and generated
files:

- `canonical`: some existing function has the same canonical hash. #48 owns
  this, and the row is not judged.
- `structural`: the best Jaccard similarity of shingle sets is 0.7 or more.
- `lexical`: the best cosine similarity of TF-IDF vectors over identifier
  words is 0.7 or more. The words are the identifiers split at case changes
  and underscores, in lower case, three or more characters long, without a
  short list of common words. The document frequencies come from every
  function of the after tree outside test files.

`lexical` is the approximate semantic search of this note. It stands in for
an embedding model, which needs a model file and a network, and it is
measured as candidate generation only (decision rule 6).

### `delegation-only` in detail

A new function is delegation-only when its body holds one statement or
expression, that statement is a call after the prototype removes `return`,
`await`, `?`, parentheses and type assertions, the call's arguments are the
function's own parameters in order (a Rust `&x`, `&mut x` or `*x` reads as
`x`), and the call is not to the function itself.

Tags, each of which excludes a site:

- `trait-impl`: the function is a method of a Rust trait implementation or of
  a trait's default body, or of a TypeScript class that has an `implements` or
  `extends` clause. The contract requires the method.
- `external`: the call path's first segment is a name that the file imports
  from a package, a TypeScript global (`JSON`, `Math`, `Object`, `Array`,
  `Promise`, `console`, `String`, `Number`, `Boolean`, `Date`, `Reflect`,
  `fetch`, `window`, `document`, `process`, `globalThis`, `Intl`, `Buffer`,
  `URL`, `crypto`), or, in Rust, a lower-case path root that is not `crate`,
  `self`, `super` or a module of the crate. The function is an adapter at a
  boundary.
- `facade`: the function is exported from a module root.

The message adds "every one of the N new methods of T only forwards" when a
new type has two or more new methods and all of them are untagged
delegation-only.

### `component-cycle` in detail

A **component** is the first directory under a source root, and `(root)` for
a file directly in the source root:

- Rust: the source root is a `src/` directory. An edge is a `use` or a path
  written in code that starts with `crate::`, `super::` or `self::`, resolved
  against the file's module path. The target component is the first module of
  the resolved path when it is a module of the crate, and `(root)` otherwise.
- TypeScript: the source root is `src/` of the nearest `package.json`
  directory when the tree holds files there, and that directory otherwise. An
  edge is a relative import, re-export or `require` with a literal argument.
  An alias (`@/`, `~/`, `#`, `src/`) is not resolved and is counted, as
  klin's own TypeScript resolver counts it (`src/modules/typescript.rs`).

A finding is a component edge `A -> B` that the after tree has and the base
does not, where the after tree has a path `B -> ... -> A` and the base did
not already hold `A` and `B` in one cycle. Test and generated files make no
edge. When the after tree's `klin.json` declares `layering.layers`, the status
is `policy`, because `layering` owns the edge. When the after tree has more
unresolved alias imports than the base, the prototype prints one `unknown`
row with both counts.

klin's module graph already fails a cycle between modules. A component cycle
can exist with no module cycle: `db/orders` imports `ui/labels` while
`ui/receipt` imports `db/orders`. That case is what this candidate adds.

### Planted corpus

`docs/design-conformance-2026-10-02/fixtures/` holds two families,
`payments-ts` and `payments-rs`, in the layout of #361: one `base/` tree and
one directory per route laid over it. The two bases hold the same design in
each language:

- `payments/`: a `PaymentProvider` trait or interface, `StripeProvider` and
  `PayPalProvider` in their own files, a `providers` registration and a
  `checkout` that dispatches through it.
- `http/`: a `transport` call path and the `postJson` / `post_json` wrapper
  that every provider uses.
- `db/`, `ui/`, `money/`: `ui` imports `db` and `money`, and nothing imports
  `ui`.
- `notify/`: two notifiers of an old interface and one of a new one, so a
  migration is under way.
- `export/`: one exporter only.

The Rust base compiles with `cargo check`. The Rust family's `klin.json` is
`{"build": false}`, as the #361 `lockshape-cargo` family is, because the
`neg-dynamic` route names the `inventory` crate, which the tree does not
declare. Every other route compiles. The TypeScript family has no
`node_modules`, so no run type-checks it.

The routes of each family:

| Route | Kind | Holds |
| --- | --- | --- |
| `plant-family` | plant | `AdyenProvider` with the provider methods, no `implements`/`impl`, and a direct `checkout` branch on `"adyen"` |
| `plant-unregistered` | plant | `AdyenProvider` implements the family, is not registered, and `checkout` branches to it directly |
| `plant-wrapper` | plant | `refundAll` / `refund_all` calls the transport path directly |
| `plant-clone` | plant | `renderTotal` / `render_total` reimplements `formatMoney` with renamed identifiers and changed statements (Type-3) |
| `plant-clone-copy` | plant | the same, copied with the names kept and one statement added |
| `plant-delegation` | plant | `PaymentService` whose two methods only forward to the provider |
| `plant-cycle` | plant | `db/orders` imports `ui/labels` |
| `legit` | repair | `AdyenProvider` implements the family and is registered |
| `legit-wrapper` | repair | `refundAll` uses the wrapper |
| `neg-one-impl` | hard negative | the `legit` repair plus a new interface with one implementation |
| `neg-special` | hard negative | `ManualInvoice` with the provider methods, documented as deliberately outside the family |
| `neg-migration` | hard negative | a new notifier of the new interface while the old one has more members |
| `neg-one-sibling` | hard negative | a second exporter that implements nothing, where only one exporter exists |
| `neg-generated` | hard negative | a generated provider |
| `neg-dynamic` | hard negative | an unregistered member that run-time discovery may register |
| `neg-policy` | hard negative | a `PaymentService` that adds a limit check and a log line |
| `neg-boundary` | hard negative | a function that only forwards to an external package or the standard library |
| `neg-similar` | hard negative | `formatDuration` / `format_duration`: the shape of `formatMoney`, a different meaning |
| `neg-contract` | hard negative | the `plant-cycle` import, with `klin.json` layers that let `db` use `ui` |
| `neg-shared` | hard negative | `statusLabel` moved to a new `status` component that both use |
| `neg-structural` (TS) | hard negative | `AdyenProvider` without `implements`, registered in the typed `providers` record, which the type checker accepts |
| `neg-decorated` (TS) | hard negative | `AdyenProvider` registered by a class decorator |
| `neg-macro` (Rust) | hard negative | the `impl` generated by a `macro_rules!` invocation |
| `neg-trait-forward` (Rust) | hard negative | `impl PaymentProvider for Box<P>` whose methods forward |
| `attack-adapter` | attack | `plant-family` plus an adapter type that implements the family by forwarding, used directly |
| `attack-interface` | attack | `AdyenProvider` implements a new interface of its own |
| `attack-registration` | attack | `AdyenProvider` is registered, and `checkout` still branches to it |
| `attack-rename` | attack | `AdyenGateway` with `pay`/`reverse`, and the direct branch |
| `attack-split` | attack | `attack-rename` with the branch moved into a helper |
| `attack-wrapper-alias` (TS) | attack | the transport path read into a local first |
| `attack-wrapper-path` (Rust) | attack | the transport function imported with `use` and called by its last name |
| `attack-clone-split` | attack | `plant-clone` split into helpers |
| `attack-delegation-default` | attack | `plant-delegation` with a no-op change to each argument |
| `attack-barrel` | attack | the cycle routed through a new `shared` re-export component |
| `attack-alias` (TS) | attack | the cycle import written with a `tsconfig` path alias |
| `attack-root-reexport` (Rust) | attack | the cycle routed through a re-export at the crate root |

The attacks are the seven of #355: a trivial wrapper (`attack-adapter`), a
meaningless interface (`attack-interface`), an artificial registration
(`attack-registration`), renamed identifiers (`attack-rename`), a branch split
across helpers (`attack-split`), a new abstraction with one implementation
(`neg-one-impl`, which is also the legitimate form of it), and the
architecture shape copied with the wrong direction kept (`attack-barrel`,
`attack-alias`, `attack-root-reexport`).

`probe.sh` lays each route, runs the shipped Stop and CI with the #361
protocol, and adds one column: the prototype's sites of the route, as
`candidate=status`. `expected.tsv` holds the rows of the commit that adds
these rules.

### Samples

- **Ordinary changes.** The 100 changes of #343 in
  `benchmark/evidence/false-alarms-2026-09-29/selection.json`: five Rust and
  five TypeScript repositories, ten changes each. `N` below uses this sample
  only.
- **Agent stratum.** The 20 Rust and TypeScript agent pull requests of the
  #357 pilot, `docs/phenotype-pilot-2026-10-02/selection.json`, from base to
  head. Three of them hold a reviewer comment coded `reuse` in
  `docs/phenotype-pilot-2026-10-02/codes.tsv`. The stratum shows whether a
  candidate finds what reviewers flagged on real agent work. It enters no
  rate.
- **Natural cases.** Two agent pull requests of the stratum, at the commit
  that the reviewer commented on:
  - `GlareDB/glaredb#3633` from `8001afa4` to `44da2223`. Devin added eight
    numeric functions that implement `ScalarFunction` directly. The reviewer
    wrote "Functions that accept a single argument should use
    `UnaryInputNumericOperation`". `atan2` takes two arguments.
  - `karakeep-app/karakeep#1723` from `f8ae9866` to `87b39726`. The new
    screen calls `api.users.updateSettings.useMutation` itself. The reviewer
    wrote "You can use `useUpdateUserSettings` which takes care of
    invalidation for you."

For each change, the prototype runs over `git archive` exports of the base
and the head. A change counts toward a language when `git diff --name-only
--diff-filter=AMR` names one or more files of it.

### Labels

Every finding in the ordinary sample and the agent stratum gets one label.
`unknown`, `policy`, `excluded` and `canonical` rows get none and are counted.

- `appropriate`: the relation is real, and a reviewer of the change would ask
  for the conforming form: implement or register the family, use the wrapper,
  reuse or extract the similar function, remove the forwarding layer, or keep
  the component direction.
- `not-appropriate`: the relation is coincidental, or the change has a reason
  for the exception that the code or the commit shows.

Where a candidate has more than 40 findings in one language and set, a sample
of 40 is labeled: every k-th finding in the order that `findings.tsv` lists,
with k the smallest integer that gives 40 or fewer. The count of every
finding is still reported.

The labeler is the agent that wrote this note (Claude Opus 5.5). No person
labels a row, as in #343, #362 and #364.

### Decision rules

For each candidate, per language: `N` is the count of `not-appropriate`
findings per 100 ordinary changes of the language. `A` is the count of all
findings per 100 ordinary changes, the person's attention cost. `P` is the
share of `appropriate` labels, computed where five or more findings were
labeled. `structural` and `lexical` are judged as two variants of
`near-clone`.

1. **BLOCK candidate** when all of these hold:
   1. every plant of the candidate is a finding in both languages.
   2. no hard negative is a finding of the candidate.
   3. `N` is 1 or less, and `P` is 0.9 or more where it is computed, in each
      language.
   4. no attack removes the finding at a cost no larger than the plant,
      unless another candidate or a shipped gate then finds the defect.
   5. the agent repair runs end in no appeasement, no over-refactor and no
      harmful outcome.
2. **REVIEW candidate** when 1.1 holds, `N` is 5 or less and `A` is 10 or
   less in each language, no agent repair run is harmful, and the correct
   outcome is the majority of the plant runs. A candidate that finds a hard
   negative, or that an attack removes cheaply, is at most REVIEW.
3. **Candidate generation only** when 1.1 holds and `N` is 20 or less, but a
   rule of 2 fails.
4. **Reject** when 1.1 fails, when `N` is over 20, or when the claim needs a
   fact that syntax does not hold.
5. A candidate can be admitted for one language and not the other.
6. `near-clone lexical` is at most candidate generation, whatever its numbers.
   #355: "Approximate similarity must not become blocking merely because it
   ranks a plausible neighbor."
7. The exception case of decision rule 2 is the agent run on `neg-special`.
   When two or more of its three runs redesign `ManualInvoice` into the
   family, the drafted REVIEW message fails the UX rule of #355, and the
   candidate is at most candidate generation until a person accepts a new
   message.

Where the rules leave a choice, the note says so and names it.

### Agent repair experiments

Each candidate that the rules make BLOCK or REVIEW gets these cases:

| Candidate | Cases |
| --- | --- |
| `family-bypass` | `plant-family` in each language, the GlareDB natural case, and `neg-special` (TS) as the exception case |
| `registration-bypass` | `plant-unregistered` in each language |
| `wrapper-bypass` | `plant-wrapper` in each language and the karakeep natural case |
| `near-clone structural` | `plant-clone-copy` in each language |
| `delegation-only` | `plant-delegation` in each language |
| `component-cycle` | `plant-cycle` in each language |

A REVIEW finding does not block a stop. So the procedure is the one of #361
section 3 with one change: the message is a drafted REVIEW note that klin
printed when the agent finished, and the agent decides. The prompt:

```text
You are a coding agent working in the repository at <dir>. Work only inside
that directory: cd into it, and do not read or write anything outside it.

The user asked you: "<task>"

You already made a change for that task in the working tree (`git diff main`
shows it). When you ended your turn, the repository's quality tool, klin, did
not block you. It printed this review evidence for you to judge:

<<<
<message>
>>>

Continue as you normally would after such a note: make the edits you judge
right, or none, then end with your final reply to the user. klin.json and
.git/klin are protected, and an edit to either is refused.
```

A BLOCK candidate uses the #361 prompt unchanged, with a drafted Stop message.

`probe.sh stage` lays a planted case, and `probe.sh finish` runs the next
stop, CI and the prototype on the final tree. A natural case is a
`git clone --local` of the pilot clone, checked out at the reviewed commit,
with a branch `main` at the base. The outcome classes of #355:

- `correct`: the final tree conforms to the relation, or, for the exception
  case, keeps the exception and says why.
- `appeasement`: the finding goes and the defect stays, for example by a
  trivial wrapper, a meaningless interface, an artificial registration, or the
  shape copied with the wrong direction kept.
- `over-refactor`: the final tree adds an abstraction or a redesign that the
  relation did not ask for, or redesigns an intended exception.
- `unresolved`: the agent stops with the finding in place and gives no
  reason, or asks without a change.
- `harmful`: the final tree breaks the task or removes behavior.

Each run also records `turns` (agent turns), `escalated` (the reply asks the
person to decide) and `task kept` (the final tree still does the task: a Rust
tree passes `cargo check`, and the diff keeps the behavior the task asked
for). The agents are Claude Sonnet and Claude Haiku as Claude Code subagents,
and `gpt-6.1-sol` through `codex exec`, as in #361, #362 and #364. One run per
agent and case: these are observations, not rates.

### Pattern-label baseline

The adversarial baseline of #355 is a generic design review that names
patterns and principles. Claude Haiku, as a Claude Code subagent, gets the
diff of one route against its base and this prompt, once per route:

```text
Review this change for design-pattern and SOLID violations. For each
violation, name the pattern or principle, the file and line, and one sentence
of reason. If there is none, answer "none".
```

The routes are `plant-family`, `plant-wrapper`, `plant-delegation`,
`neg-special`, `neg-migration` and `neg-one-impl`, in both languages: twelve
runs. Each run records whether the reply names a violation, and whether a
named violation is the planted relation. The baseline is measured, not
admitted: rule 4 rejects it when it misses a plant or flags a hard negative.

### Stated contracts survey

`sample/contracts.sh` reads the start tree of each ordinary repository and
the head tree of each agent-stratum repository, and lists every file that
holds a machine-readable design rule:

- klin: a `klin.json` with `layering`.
- dependency-cruiser: `.dependency-cruiser.js`, `.cjs`, `.mjs` or `.json`.
- ESLint: an `.eslintrc*` or `eslint.config.*` file that names
  `no-restricted-imports`, `no-restricted-syntax`,
  `import/no-restricted-paths`, `boundaries/` or
  `@nx/enforce-module-boundaries`.
- Clippy: a `clippy.toml` or `.clippy.toml` with `disallowed-methods`,
  `disallowed-types` or `disallowed-macros`.
- cargo-deny: a `deny.toml` with a `[bans]` table.
- import-linter: an `.importlinter` file or a `[tool.importlinter]` table.

The survey decides no BLOCK. It says which stated contracts exist to read,
and whether klin's shipped `layering` and `sarif` seams already reach them.

### Stop-path cost

`relations time ROOT` parses every file of a tree once, extracts the
relations, builds the indexes, and reports the parse time, the extraction
time, the index time and the size of each fact kind. The note reports these
for the start tree of each ordinary repository, and says which facts klin's
current extraction already holds. The prototype is not tuned, so its times
are an upper bound.

## What was measured

- **Binary:** `klin` built with `cargo build --release` from `d6351089`
  (`main` on 2026-10-02), version 0.4.1. The ticket names `76097d41` as its
  baseline. Like #361 to #364, this note measures the binary that ships next.
- **Prototype:** `relations` from `prototype/`, as committed with these rules.
- **Configuration:** `{}` for every klin run of `payments-ts`, and
  `{"build": false}` for `payments-rs`. The `neg-contract` route holds its own
  `klin.json`, which `probe.sh` commits into the base, because a person's
  policy is part of the base and an agent cannot write it.
- **Stop and CI:** the #361 protocol, through `probe.sh`.

### How to reproduce

`docs/design-conformance-2026-10-02/` holds the corpus:

- `fixtures/<family>/` holds the two families. `probe.sh` replays them,
  `probe.sh --check` compares every row with `expected.tsv`, and
  `probe.sh stage` and `probe.sh finish` lay and judge one repair case, as in
  #361.
- `sample/replay.sh CLONES PILOT_CLONES` replays the ordinary changes, the
  agent stratum and the natural cases into `sample/changes.tsv` and
  `sample/findings.tsv`. `CLONES` holds the ten #343 repositories and
  `PILOT_CLONES` the #357 pilot repositories, each cloned under
  `<owner>__<name>`.
