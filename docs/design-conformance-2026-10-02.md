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
- `sample/contracts.sh CLONES PILOT_CLONES` writes `sample/contracts.tsv`.
- `relations time ROOT` on each start tree wrote `sample/time.tsv`.
- `sample/languages.tsv` counts the Rust and TypeScript files of each
  ordinary change. `sample/labels.tsv` holds one label per labeled finding.
- `experiments.tsv` lists the repair cases, `runs/messages/` the drafted
  REVIEW notes, `runs/<agent>/<case>.diff` each final tree (a planted case
  against its base, a natural case against the reviewed commit, lockfiles and
  `target/` left out), `runs/codex/*.reply` the codex replies,
  `runs/outcomes.tsv` the outcome of every run and `runs/baseline.tsv` the
  twelve pattern-label replies.

### Changes after the rules were registered

No prototype change and no corpus change came after the rules commit
(`709ac685`). These changes to the procedure did:

1. **Prompt delivery.** Each Claude subagent got a one-line message that told
   it to read its full prompt from a file and follow it. The file holds the
   registered prompt text exactly. Codex got the text directly.
2. **Second turn.** Sonnet's `cycle-rs` run removed the public `ui::labels`
   module, and the shipped `public-api` gate blocked the next stop. As in
   #361, that agent got the block message and one more turn.
3. **Diffs.** The first diffs held Rust `target/` files that agents built.
   The judge then left `target/` out. A natural case's diff is against the
   reviewed commit, so it shows only the agent's own edits.
4. **Labels.** Every finding was labeled, except `component-cycle` in Rust:
   49 findings, so every second one (24) was labeled by the k-th rule.
5. **Type checks.** No tree had `tsc` when the agents ran, so no agent
   type-checked a TypeScript tree. After the PR review, the judge installed
   TypeScript 5.6.3 and `@types/node` in a scratch directory and ran
   `tsc --noEmit -p tsconfig.json --types node` on every TypeScript fixture
   tree: the base, five planted routes and the 15 final trees. Each gave 0
   errors, and a planted type error in a copy of the base gave 1. The GlareDB
   trees did not compile, because `glaredb_core` depends on `glaredb_proto`,
   whose build needs `protoc`, which is absent. The karakeep trees were not
   type-checked, because that needs the monorepo's own install. For those 6
   trees, `task kept` rests on the diff alone.
6. **Prototype fixes after the review of this note.** A review found four
   places where `relations` did not follow these rules, and the prototype now
   follows them: file-level conformance applied to every reason, not only to
   `shape`, a branch was checked against the family's own registrations, not
   against every registration, the layer message counted `facade` methods,
   and the common methods of a family skipped implementations with no
   methods. `expected.tsv` did not change. The replay of the sample with the
   fixed prototype gave the same `findings.tsv` and `changes.tsv`, byte for
   byte.
7. **Registration families.** `registration-bypass` groups a family across
   the whole tree (two or more non-test member types), not per directory as
   the Terms define a family, because a registration often sits in another
   directory than its members. The prototype did this from the rules commit
   on. The rules text did not say it.
8. **Changes after the PR review.** The review of PR #444 (comment
   5960721421) found three overstatements. This note now says them:
   - The family identity of `registration-bypass` is the last segment of a
     trait name, grouped across the whole tree. So two unrelated traits of the
     same name in two packages make one family. Resolved bindings alone do not
     fix that (section 6).
   - The `unknown` row of `component-cycle` compares only the count of
     unresolved alias imports. An alias that goes away while another comes in
     leaves the count equal, and no `unknown` row appears (section 6).
   - The outcome classes did not separate a repair that builds from one that
     nobody built. `runs/outcomes.tsv` and section 8 now use these classes:
     `relation repair, builds`, `relation repair, build unverified`, `kept
     exception`, `appeasement`, `over-refactor`, `unresolved` and `harmful`.

   No run, row or label changed. The review asked for the four positive
   candidates to be "conditional". The registered rules have no such class, so
   this note keeps their registered disposition, REVIEW candidate, and states
   each condition as a requirement that an implementation must meet before a
   person admits it (section 9).
9. **Challenge of the PR review.** A check of the review against the code and
   the data changed three of its points:
   - The yaak rows do not show two traits merged into one family.
     `UpsertModelInfo` is one trait, defined once in
     `crates/common/yaak-database/src/traits.rs`. The false relation came from
     the binding name `value`. The merging is real in the code, and no row of
     the sample comes from it.
   - In `family-bypass`, a type matched by its bare name can only make a new
     type conform. That hides a finding and never makes one. So identity
     resolution is a recall requirement for `family-bypass`, not a precision
     one.
   - klin's module graph has no coverage of TypeScript aliases to reuse. The
     resolver adds an alias to the same `external` count as a package import
     (`src/modules/typescript.rs`), and `docs/SPEC.md` keeps `tsconfig` paths
     outside V1. A TypeScript `component-cycle` needs alias resolution or one
     located hole per alias in the module graph first, and the shipped
     `layering` gate has the same blind spot (#445).
   - The judge type-checked the TypeScript trees (item 5). So 23 of 33 runs
     build, not 12, and only the 6 GlareDB and karakeep runs stay unbuilt. A
     build proves that the code compiles, not that it behaves: the fixtures
     hold no tests.

## Headline results

1. **The family relation finds real agent work, and klin can say it in the
   repository's words.** On GlareDB#3633, `family-bypass` names all 8 new
   files: "18 of 23 sibling files implement `UnaryInputNumericOperation`".
   Seven of the 8 are real bypasses. `atan2` is a real exception, because it
   takes two arguments. On karakeep#1723, `wrapper-bypass` names the hook the
   reviewer named.
2. **No candidate is a BLOCK candidate.** Every candidate that finds its
   plants also finds a hard negative or loses its finding to a cheap attack,
   or both. Renaming a type and moving one branch into a helper
   (`attack-split`) removes every finding of the family and registration
   candidates.
3. **Four candidates are REVIEW candidates, each with requirements before
   admission:** `family-bypass`, `registration-bypass` and `wrapper-bypass`,
   in both languages, plus `component-cycle` for TypeScript only. On 62
   ordinary changes, they found 0, 0, 1 and 0 findings. Each one needs a
   resolved identity or alias resolution that the prototype does not have,
   and an implementation must measure its precision again (section 9).
4. **Similarity is noise here.** `near-clone lexical` gave 85.3 not-
   appropriate findings per 100 Rust changes. Both `near-clone` variants
   missed the Type-3 plant with renamed identifiers (0.42 structural, 0.18
   lexical), while the different-meaning hard negative scored 0.71 and 1.00
   structural.
5. **The notes led agents to the repository's own repair.** Of 33 runs, 23
   repaired the relation and build (`cargo check` for Rust, `tsc --noEmit` for
   TypeScript), 6 repaired the relation in trees that nobody built (GlareDB
   and karakeep), and 4 kept an exception. No run was an appeasement or an
   over-refactor. No built run was harmful, and the 6 unbuilt runs are not
   proven harmless. A build is no behavior test: the fixtures hold no tests.
   All three agents kept `atan2` and `ManualInvoice` as exceptions.
6. **The pattern-label baseline names the wrong things.** It flagged a
   "violation" in 5 of 6 hard negatives (for example "DIP: depends on the
   global `console`"), and missed the wrapper in both wrapper plants.

## 3. Corpus and samples

The planted corpus holds 66 routes: 33 per family, each with a `base` row.
`expected.tsv` holds every row. klin's shipped gates pass every route at both
stops and in CI, because no shipped gate states these relations. The Rust `plant-clone-copy` route was first 26
lines long and klin's `complexity` gate blocked it, so the route was made two
lines shorter before the rules commit.

The ordinary sample: 100 changes, of which 34 touch Rust and 28 touch
TypeScript (`sample/languages.tsv`). 41 changes touch neither. The agent
stratum: 20 pull requests with 107 Rust and TypeScript files. The replay of
all 122 changes took 31 minutes, most of it `git archive` of the large
trees.

## 4. Results per candidate

### Planted corpus

`F` is a finding, `U` an `unknown` row, `P` a `policy` row, `-` nothing. A
row for a language that has no such route is blank.

| Route | `family-bypass` TS / RS | `registration-bypass` TS / RS | `wrapper-bypass` TS / RS | `near-clone structural` TS / RS | `delegation-only` TS / RS | `component-cycle` TS / RS |
| --- | --- | --- | --- | --- | --- | --- |
| `plant-family` | F / F | F / F | | | | |
| `plant-unregistered` | - / - | F / F | | | | |
| `plant-wrapper` | | | F / F | | | |
| `plant-clone` | | | | - / - | | |
| `plant-clone-copy` | | | | F / F | | |
| `plant-delegation` | - / - | | | | F / F | |
| `plant-cycle` | | | | | | F / F |
| `legit`, `legit-wrapper` | - / - | - / - | - / - | | | |
| `neg-special` | **F / F** | | | | | |
| `neg-migration` | **F / F** | | | | | |
| `neg-structural` | **F** / | | | | | |
| `neg-decorated` / `neg-macro` | U / U | | | | | |
| `neg-dynamic` | | U / U | | | | |
| `neg-one-impl`, `neg-one-sibling`, `neg-generated` | - / - | - / - | | | | |
| `neg-policy`, `neg-boundary`, `neg-trait-forward` | | | | | - / - | |
| `neg-similar` | | | | **F / F** | | |
| `neg-contract` | | | | | | P / P |
| `neg-shared` | | | | | | - / - |

`lexical` found `plant-clone-copy` in both languages and nothing else of the
corpus.

### Ordinary sample and agent stratum

`N` and `A` are per 100 ordinary changes of the language (34 Rust, 28
TypeScript). `P` is computed where five or more findings were labeled. For
`component-cycle` in Rust, 24 of 49 findings were labeled, and `N` applies the
share of the 24 (all not-appropriate) to all 49. The 24 labels alone give 70.6.

| Candidate | Rust findings | Rust `N` | Rust `A` | Rust `P` | TS findings | TS `N` | TS `A` | TS `P` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `family-bypass` | 0 | 0 | 0 | - | 0 | 0 | 0 | - |
| `registration-bypass` | 0 | 0 | 0 | - | 0 | 0 | 0 | - |
| `wrapper-bypass` | 1 | 2.9 | 2.9 | - | 0 | 0 | 0 | - |
| `near-clone structural` | 6 | 5.9 | 17.6 | 0.67 | 0 | 0 | 0 | - |
| `near-clone lexical` | 36 | 85.3 | 105.9 | 0.19 | 9 | 32.1 | 32.1 | 0.00 |
| `delegation-only` | 4 | 11.8 | 11.8 | - | 13 | 46.4 | 46.4 | 0.00 |
| `component-cycle` | 49 | 144.1 | 144.1 | 0.00 | 0 | 0 | 0 | - |

What the rows hold:

- `wrapper-bypass` (Rust): `std::fs::File::open` in arnis, which the base
  called only once, inside a function that is no wrapper of it.
- `near-clone structural` (Rust, arnis): 4 real reuse cases (a second
  weighted-pick loop, a third varint reader, a compass-bearing parser that is
  a subset of an existing one, a second `rotate_dir`) and 2 functions of the
  same shape and another meaning.
- `near-clone lexical`: 7 of 45 are real reuse, among them a second
  `write_atomic` and a repeated `level.dat` loader. The other 38 share only
  names or common words with their neighbor.
- `delegation-only`: accessors (`self.x.clone()`, `this.map.get(k)`), named
  `AsyncLocalStorage` wrappers, a Tauri command, public API methods, and 4
  type-test files that the path rule did not mark as tests.
- `component-cycle` (Rust): every finding is in arnis, a flat crate in which
  every top-level file is a component and the base modules already form one
  tangle. Each new module that joins the tangle reads as a new cycle.

The agent stratum (20 pull requests) holds one `family-bypass` row (`atan2`
at GlareDB's final head, not-appropriate: the reviewer accepted it), one
`wrapper-bypass` row and five `delegation-only` rows. The wrapper row is
real: jdx/mise#6216 wraps the whole run in `tokio::time::timeout` itself,
while the base calls that function only inside `run_with_timeout_async`,
which 8 sites use. The five delegation rows are accessors in
anthropics/claude-code#8345.

Of the three agent pull requests that reviewers coded `reuse` in #357, two are
Rust or TypeScript. The prototype finds both at the commit that the reviewer
commented on, by the candidate that names the reviewer's own words. The
third, dify-official-plugins#1422, is Python and out of scope.

### Unknown and policy rows

- 7 `registration-bypass` rows in mountain-loop/yaak read `unknown`, because
  a TypeScript file near the branch imports code at run time. Each of them is
  a false relation: a Rust `match` in `crates/yaak/src/import.rs` binds
  `value`, and the TypeScript branches name a local `value`. Without the
  run-time marker, these 7 rows would be findings. The binding must be
  resolved through imports, in one language, before a message can claim it.
- 4 `component-cycle` rows in Open-Dev-Society/OpenStock read `unknown`: the
  tree has 134 to 245 alias imports (`@/`), which neither the prototype nor
  klin's resolver resolves. On such a tree the candidate sees almost nothing.

### Stated contracts

5 of 30 trees hold a stated design rule (`sample/contracts.tsv`):

- gfx-rs/wgpu `clippy.toml`: `disallowed-types` for
  `std::collections::HashMap` and `HashSet`, "use hashbrown::HashMap
  instead". This is a stated reuse rule.
- apollographql/apollo-client `eslint.config.mjs`: `no-restricted-imports`,
  "Please use named export `{ equal }` from @wry/equality instead", and a
  `no-restricted-syntax` rule.
- tokens-studio/figma-plugin `.eslintrc.js`: `no-restricted-syntax` against
  `for...in`, labels and `with`. These are style, not design.
- gluesql/glues and jdx/mise `deny.toml`: `[bans]` tables for crate versions,
  not design.

No tree holds a dependency-cruiser, import-linter, `eslint-plugin-boundaries`
or Nx boundary rule, and none holds a klin `layering` section. The stated
reuse rules that exist are already enforced by the repository's own linter.

## 5. Attacks

| Attack | What it removes | Who still finds the defect | Cost of the attack |
| --- | --- | --- | --- |
| `attack-adapter` (trivial wrapper) | `family-bypass`, because the file now holds a type that implements the family | `registration-bypass`: the adapter is unregistered and the branch stays | one forwarding type |
| `attack-interface` (meaningless interface) | nothing | `family-bypass` and `registration-bypass` | none |
| `attack-registration` (artificial registration) | the `unregistered` form | `registration-bypass` (the branch) | none |
| `attack-rename` | `family-bypass` (no `*Provider` suffix, no shared methods, no shared name form) | `registration-bypass` (the branch) | renaming |
| `attack-split` (branch in a helper) | both | nothing | renaming plus one helper |
| `neg-one-impl` (one-implementation abstraction) | not an attack in its legitimate form: no candidate finds it | - | - |
| `attack-wrapper-alias` (TS) / `attack-wrapper-path` (Rust) | `wrapper-bypass` | nothing | one line |
| `attack-clone-split` | `near-clone` | nothing | splitting |
| `attack-delegation-default` | `delegation-only` | nothing | one no-op per argument |
| `attack-barrel` (wrong direction through a re-export) | nothing | `component-cycle` finds both new edges | - |
| `attack-alias` (TS) | `component-cycle`, one `unknown` row stays | nothing | a `tsconfig` alias |
| `attack-root-reexport` (Rust) | `component-cycle` | nothing | one `pub use` at the crate root |

So the cheapest route to green is a legitimate repair only for the wrong
direction through a barrel. For every other candidate, a rename, an alias or
a split removes the finding at a cost no larger than the plant. #355: "A
future gate is useful only when the cheapest obvious route to green is also a
legitimate repair." No candidate meets that rule for a gate. A REVIEW note
does not need to: it asks for a judgement, and nothing turns green.

The attacks rest on the same cause. Each candidate keys a relation by a
name or a spelling that syntax shows, not by a resolved symbol: a type suffix,
a call path's text, a binding's name, an import specifier.

## 6. Precision and measurement holes

- **Family membership by syntax.** TypeScript is typed by structure, so a
  class without `implements` can be a member (`neg-structural`). Only the type
  checker proves it. Rust macros and TypeScript decorators can create or
  register a member out of sight. The prototype reads these as `unknown`.
- **Exceptions look like bypasses.** `atan2`, `ManualInvoice` and a migration
  to a new interface are all sibling-like and outside the dominant family. The
  relation cannot tell an intended exception from a bypass.
- **Call paths by text.** `wrapper-bypass` compares written text, so a `use`
  or a local alias hides the path. It needs the callee resolved to a
  declaration, which klin's references do not hold (they hold names only).
- **Identity by name.** `registration-bypass` matched a Rust binding to a
  TypeScript identifier (yaak). The problem is deeper than the binding.
  `type_name` keeps only the last segment of a trait or type name, and
  `global_families` keys the whole tree by that string. So two unrelated
  traits named `Provider` in two packages make one family, and their members
  and registrations mix. `family-bypass` keys a family per directory, which
  limits the mixing, but its conformance check also matches an implementing
  type by its bare name anywhere in the tree. That match can only make a type
  conform, so in `family-bypass` it hides findings and never makes a false
  one. The yaak rows are not an instance of the merging: `UpsertModelInfo` is
  one trait, defined once, and the false relation came from the binding name
  `value`. No row of the sample comes from two merged traits. Both candidates
  need `implements(resolved type, resolved trait)`: for `registration-bypass`
  for precision, for `family-bypass` for recall. `registration-bypass` also
  needs its member names and bindings resolved to declarations.
- **Unresolved edges by count.** `component-cycle` prints `unknown` only when
  the after tree has more unresolved alias imports than the base. An alias
  that goes away while another comes in leaves the count equal. An alias that
  already existed and now closes a cycle also leaves it equal. In both cases
  no `unknown` row appears, and the candidate reads a partial graph as a
  complete one. On trees with 134 to 245 alias imports, this is the common
  case. A hidden base edge can also make an old cycle look new. klin's module
  graph has the same gap: its TypeScript resolver adds an alias to the same
  `external` count as a package import, and `docs/SPEC.md` keeps `tsconfig`
  paths outside V1. So a shipped version needs alias resolution, or one
  located hole per alias, in the module graph first. Then an unresolved
  import in the changed files, or on the path of a cycle, makes the result
  unknown, whatever the count. The shipped `layering` gate has the same blind
  spot on trees that use aliases.
- **Components by directory.** A flat Rust crate makes every module a
  component, and Rust modules of one crate may depend on each other freely. In
  TypeScript, path aliases hide most edges of some trees.
- **Similarity.** Shape similarity does not track meaning: a different-meaning
  function scored 1.00, a renamed reimplementation 0.42. Name similarity
  depends on naming, which the attack controls.
- **Labels.** One agent labeled every row, and no person reviewed them, as in
  #343, #362 and #364.

## 7. Performance and architecture

`sample/time.tsv` holds `relations time` on each start tree. The prototype
parses every file and extracts every relation in one pass, untuned:

| Tree | Files | Parse ms | Extract ms | Impl bytes | Call bytes | Registration bytes | Shingle bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| gfx-rs/wgpu | 855 | 740 | 2,828 | 356,055 | 1,056,609 | 695,818 | 3,786,128 |
| refactoringhq/tolaria | 1,695 | 841 | 2,338 | 35,077 | 321,280 | 71,463 | 2,821,632 |
| apollographql/apollo-client | 760 | 752 | 2,462 | 7,152 | 51,862 | 295,938 | 699,752 |
| mountain-loop/yaak | 1,061 | 357 | 1,106 | 140,926 | 236,290 | 118,831 | 1,874,248 |
| denisidoro/navi | 52 | 9 | 27 | 4,111 | 2,505 | 1,822 | 52,472 |

Extraction costs 0.5 to 14.4 ms per file: 14.4 on pdf-inspector, whose files
are large, 8.2 on arnis, 3.3 on wgpu and 1.0 to 1.5 on the large TypeScript
trees. Most of it is the shingle walk of `near-clone`, which the dispositions
reject.

What each REVIEW candidate needs:

| Candidate | Fact | In `FileFacts` now? | Base evidence | Affected closure |
| --- | --- | --- | --- | --- |
| `family-bypass` | `implements(type, trait)`, a type's methods, top-level names | No. A trait implementation's methods carry no `owner`, and no fact names the trait. Declarations and names exist. | per directory: about 4 to 360 KB on these trees | the changed file's directory |
| `registration-bypass` | `implements`, plus literal collections that name types, plus resolved references to their bindings | No | whole tree: 2 to 700 KB | every function that reads a family's registration |
| `wrapper-bypass` | a call's resolved callee and its enclosing declaration | No. References hold a name and a line. | whole tree: 3 KB to 1.1 MB of specific calls | the base callers of each changed call |
| `component-cycle` (TS) | resolved module edges | Yes: the module graph of `layering` | the module graph | the whole graph |

None of these candidates needs a second parser: one pass over the tree-sitter
tree that klin already builds gives every fact. Each one needs base evidence
of the whole tree or of a directory. A REVIEW note is not judged at each Stop.
So the work belongs at Finalize, once per change, where a warm run over 20
changed files would read its directories and the cached base facts. This note
does not measure a klin-integrated warm run. An estimate from the prototype:
20 changed files cost 20 times the per-file extraction, 10 to 290 ms on these
trees, plus the index time of `sample/time.tsv` (0.1 to 339 ms) where the base
evidence is not cached. That is an upper bound, because the shingle walk is
in it.

## 8. Agent repair experiments

`runs/outcomes.tsv` holds every run. Each outcome rests on the final tree,
which the judge read as a diff and ran through the prototype and klin, then
through `cargo check` (Rust fixtures) or `tsc --noEmit` (TypeScript
fixtures). A diff shows whether a repair has the right shape for the
relation. A build shows that the code still compiles. Neither shows that it
behaves, because the fixtures hold no tests. So a run whose tree builds is
`relation repair, builds`, and a run whose tree nobody built is `relation
repair, build unverified`. The corpus does not keep the Claude
replies, as in #361. A reply only adds the `escalated` column and, where an
agent kept code, its stated reason. Eleven cases, three agents:

`builds` is `relation repair, builds`, `unbuilt` is `relation repair, build
unverified`, `kept` is `kept exception`.

| Case | Sonnet | Haiku | gpt-6.1-sol |
| --- | --- | --- | --- |
| `family-ts` | builds | builds | builds |
| `family-rs` | builds | builds | builds |
| `family-glaredb` | unbuilt, `cot` now returns `-inf` for `-0`, stated | unbuilt | unbuilt |
| `exception-ts` | kept, said why | kept, rewrote the comment | kept, said why |
| `registration-ts` | builds (the reply was only "placeholder") | builds | builds |
| `registration-rs` | builds | builds | builds |
| `wrapper-ts` | kept: "a retried refund POST could refund twice" | builds | builds |
| `wrapper-rs` | builds | builds | builds |
| `wrapper-karakeep` | unbuilt | unbuilt | unbuilt |
| `cycle-ts` | builds | builds | builds |
| `cycle-rs` | builds, in 2 turns | builds | builds |

| Outcome | Sonnet | Haiku | gpt-6.1-sol | All |
| --- | ---: | ---: | ---: | ---: |
| relation repair, builds | 7 | 8 | 8 | 23 |
| relation repair, build unverified | 2 | 2 | 2 | 6 |
| kept exception | 2 | 1 | 1 | 4 |
| appeasement | 0 | 0 | 0 | 0 |
| over-refactor | 0 | 0 | 0 | 0 |
| unresolved | 0 | 0 | 0 | 0 |
| harmful | 0 | 0 | 0 | 0 |
| escalated to the person | 0 | 0 | 0 | 0 |
| extra turns | 1 | 0 | 0 | 1 |

`harmful` reads 0 only for what the judge could see. No run broke a build,
and no diff removed behavior that the task asked for. The 6 unbuilt runs may
still break the build, and no run had a test that could show broken
behavior. This note does not claim otherwise.

On GlareDB, all three agents made the repair that the pull request itself
made after the review (`5f4ac7d4`): seven functions moved to
`UnaryInputNumericOperation`, and `atan2` stayed. No agent redesigned
`ManualInvoice`, so decision rule 7 holds.

Two observations matter for the message:

- **A wrapper carries policy.** `postJson` retries 3 times. Five of six
  planted wrapper runs moved `refundAll` onto it, so refunds now retry. The
  base's Stripe and PayPal refunds already go through `postJson`, so those
  repairs match the repository, and this note counts them as repairs of the
  relation. Sonnet's
  TypeScript run kept the direct call, because a retried refund POST could
  refund twice. Its Rust run named the same risk and switched. Both are
  defensible. The relation cannot tell whether new code avoids the wrapper on
  purpose. That is the reason for REVIEW and not BLOCK.
- **A kept exception outside the exception case.** The registered classes
  give `correct` to a kept exception only in the exception case. So the one
  kept wrapper run is `kept exception` and is not counted as a repair. Rule 2
  holds with or without it: 8 of 9 wrapper runs repaired the relation.
- **The note was enough.** No run asked the person. No run added an interface,
  a wrapper or a registration that only silences the note.

The only shipped gate that acted was `public-api`, on Sonnet's `cycle-rs`
run. The repair moved `status_label` out of `ui` and removed `ui::labels`,
and the second turn kept the old path as a re-export.

### Pattern-label baseline

`runs/baseline.tsv` holds the twelve replies:

| Route | TS: names a violation / names the planted relation | Rust |
| --- | --- | --- |
| `plant-family` | yes / yes ("two competing selection mechanisms") | yes / yes |
| `plant-wrapper` | yes / no (DIP, SRP, OCP, `postJson` not named) | yes / no |
| `plant-delegation` | yes / yes ("adds no behavior beyond delegation") | yes / yes |
| `neg-special` | yes (DIP, "factory anti-pattern") | yes (DIP, SRP) |
| `neg-migration` | yes (DIP for `console`) | no |
| `neg-one-impl` | yes (OCP for the registry edit) | yes (SRP) |

The baseline found the relation where the diff itself shows it, and missed it
where the relation lives in the base (the wrapper). It named a violation in 5
of 6 hard negatives. A message that says "violates DIP" for a `console.info`
call is the bad output that #355 describes.

## 9. Disposition per candidate

| Candidate | Disposition | Rule that decides it |
| --- | --- | --- |
| `family-bypass` | **REVIEW candidate**, Rust and TypeScript. Requirement: resolved type and trait identity, for recall | 1.1 holds, `N` 0, `A` 0, 9/9 plant and natural runs repaired the relation (6 build) and 3/3 exception runs kept it, hard negatives found and `attack-split` removes it, so not BLOCK |
| `registration-bypass` | **REVIEW candidate**, Rust and TypeScript. Requirement: resolved family, member and binding identity, and a new precision measurement after it | 1.1 holds, `N` 0, `A` 0, 6/6 runs repaired the relation (all build), `attack-split` removes it, the yaak rows show a false relation from a binding name |
| `wrapper-bypass` | **REVIEW candidate**, Rust and TypeScript. Requirement: resolved callees, which an alias or a `use` otherwise hides | 1.1 holds, `N` 2.9 (Rust) and 0, `A` under 10, 8 of 9 runs repaired the relation (5 build) and 1 kept an exception, one line of alias removes it |
| `component-cycle` | **REVIEW candidate** for TypeScript. Requirement: alias resolution or one located hole per alias in the module graph, which V1 does not have, and a new precision measurement after it. **Reject** for Rust | TS: `N` 0, 3/3 `cycle-ts` runs repaired the relation (all build), and `N` 0 is no measure of precision while aliases hide edges, Rust: `N` 144.1 |
| `near-clone structural` | **reject** | 1.1 fails: the Type-3 plant is missed in both languages |
| `near-clone lexical` | **reject** | 1.1 fails, and `N` 85.3 (Rust), 32.1 (TS) |
| `delegation-only` | **candidate generation only** for Rust, **reject** for TypeScript | Rust `N` 11.8, TS `N` 46.4 |
| pattern-label baseline | **reject** | flags 5 of 6 hard negatives, misses both wrapper plants |
| stated contracts (R7) | survey only, no native candidate: the reuse rules found (wgpu, apollo-client) are already rules of the repository's own linter, which klin can read through the shipped `sarif` seam or a #363 recipe | rule 4: a native check would restate a fact the linter already holds |

Choices for a person:

- `near-clone structural` found 4 real reuse cases in 6 Rust rows. It is
  rejected by rule 1.1, because it misses the renamed reimplementation. A
  person may choose to keep it as candidate generation for a reviewer, which
  the registered rules do not allow.
- `wrapper-bypass` would need a stated exception form (for example a comment
  that names the wrapper) before it is prominent. This note does not test one.
- The REVIEW candidates found almost nothing on 62 ordinary human changes and
  found the reviewer's point on both natural agent cases. Whether that yield
  justifies a Finalize step is a product decision.
- The four REVIEW candidates go to #357 as candidates, not as admitted
  REVIEW checks. Each requirement above must hold, and an implementation must
  measure its precision again inside klin, before a person admits one.

## 10. New shared structural fact

No new shared fact is justified by this note alone. Each relation below has
only REVIEW candidates as consumers, and each of them runs at Finalize, not at
Stop. If a person admits the candidates, the implementation ticket needs these
named relations, each keyed by a resolved identity and never by a bare name:

| Relation | Consumers | Why it is not in `FileFacts` today |
| --- | --- | --- |
| `implements(resolved type identity, resolved trait identity)`: a Rust `impl F for T` and a TypeScript `implements`/`extends` clause, with the method names | `family-bypass`, `registration-bypass` | a trait implementation's methods carry no `owner`, no fact names the trait, and the prototype's last-segment name merges unrelated traits |
| a call's resolved callee and its enclosing declaration | `wrapper-bypass`, `registration-bypass` (the members a registration names and the binding a branch reads) | a `Reference` holds a name and a line, not a path or a target |

`component-cycle` for TypeScript needs no new structural fact, but it needs a
change to the module graph of `layering`: alias resolution (`tsconfig`
paths), or one located hole per alias in place of the shared `external`
count. That change has a shipped consumer of its own, `layering`, which has
the same blind spot today. The registration expression (a literal list of
type names) is local to `registration-bypass` and is not a shared fact.

## 11. SPEC language, if a person admits the REVIEW candidates

> **Design-conformance evidence is review evidence.** At Finalize, klin may
> report a change that bypasses a relation the base already states: a new
> sibling type that does not implement the trait or interface that two or more
> sibling files implement, a new member or branch that a family's registration
> does not hold, or a new call of a path that the base calls only inside one
> exported wrapper. A note names the existing relation, the new relation and
> why klin cannot decide: the members, the registration or the wrapper with
> their locations, and the count. It never names a design pattern or a
> principle, and it never fails a Stop or CI. A relation that a decorator, a
> macro or run-time discovery may form reads as unknown and makes no note. A
> relation keyed by an unresolved name, or a module graph with an unresolved
> import in the changed files or on the cycle, reads as unknown and makes no
> note. A
> person who keeps an exception needs no configuration: the note asks for a
> reason in the reply.

## 12. UX, DX and AX

**UX.** Each note says "Why klin is unsure" and names the exception it
cannot rule out. In all 6 runs that met a planted or natural exception, the
agent kept it: the 3 `exception-ts` runs and `atan2` in the 3 GlareDB runs.

**DX.** The minimum evidence of a note is: the existing relation set
("StripeProvider (stripe.ts) and PayPalProvider (paypal.ts) implement
PaymentProvider"), the new relation ("AdyenProvider ... does not implement
PaymentProvider"), the unresolved part (decorators, macros, run-time
discovery read as unknown, so no note), and the reason it is REVIEW (the
exception sentence). None of it needs a graph word.

**AX.** 29 of 33 runs made the repository's own repair, 23 of them in a tree
that builds, and 4 kept an exception. The only shipped gate that acted was
`public-api`, which caught a removed module in one repair.

## Limits

- Two planted families, written by the same agent that wrote the prototype.
- 62 ordinary changes in Rust or TypeScript, 20 agent pull requests and 2
  natural cases. Zero findings on the ordinary sample is a small base for a
  precision claim.
- One run per agent and case. The REVIEW prompt says that nothing blocks, so
  the runs show how agents treat a note, not how they treat a block.
- The messages were drafted by hand in the form the prototype's rows allow.
  A shipped message may only claim what its relation shows.
- The prototype is not klin. It reads the same grammars, but none of its
  facts comes from klin's extractor.

## Decision

**Keep design and reuse conformance beyond #48 as Finalize review evidence.**

No relation in this note can block: each one has an exception that syntax
cannot rule out, and each one falls to a rename, an alias or a split at no
more cost than the plant. Three relations (`family-bypass`,
`registration-bypass`, `wrapper-bypass`) and the TypeScript component cycle
name the repository's own relation, and every agent made the relation's
repair or kept a stated exception. They go to #357 as REVIEW candidates, not
as admitted REVIEW checks: each one needs resolved identities (`implements`
over resolved types and traits, resolved callees and bindings) or alias
resolution in the module graph, and an implementation must measure its
precision again. Similarity search, delegation counting
and pattern labels do not qualify. The implementation of that review step is
a separate ticket, which a person decides.
