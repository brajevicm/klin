# Public API relaxes a strict contract only from evidence

> Amends ADR 0044, section "Judgement, holes and the failure model". Surfaces,
> items, identities, holes and the accepted list stand. What changes is how
> two measured contracts are compared, and which facts a contract shows.

ADR 0044 failed every changed measured contract. The seeded benchmark run
b647fbfa8993 showed the cost: klin failed
`interface Point { height?: number; lat: number; lon: number }` against
`interface Point { lat: number; lon: number }`, and a `tsc` check of ordinary
object-literal and caller use compiled. The 48 public-api occurrences of the
v1 and v2 rounds were all labeled `valid-review`. So the measured problem is
one kind of ordinary additive growth that klin blocked, not a lack of subtle
compatibility rules. #303 asked for this record, #301 tracks the evidence, and
#304 carries it out.

An earlier draft of this record gave each language a table of compatible
changes. An adversarial review compiled consumers against it with `tsc` 5.8.2
and `rustc` 1.98.1 and found eight changes that the tables passed or could not
see although ordinary consumer code broke: an empty interface that became a
weak type, `in` narrowing, call signatures under `strictFunctionTypes`, a
method implementer's own extra parameter, an `as` cast of a
`#[non_exhaustive]` enum, a variant-level `#[non_exhaustive]`, a private field
under `#[cfg]`, and a field that shadows one reached through `Deref`. Each fix
added a language-specific condition, and each new language would need its own
table and its own review. This record keeps the language knowledge in the
adapters and gives the gate one rule.

In ADR 0044, "widening" in "Additions, widening and an opaque item that became
measured pass" means a widened visibility. It never meant a widened type.

## What the gate is for

`public-api` is a review signal for a contract change the task did not ask
for. It does not prove semantic-version compatibility, and it does not prove
that every consumer still compiles; the build and the consumer's own CI do
that. A false negative lets a contract drift silently. A false positive costs
a block and an accepted entry, and it can push an agent to change the API only
to satisfy the gate. klin prefers the false positive whenever it cannot
classify a change narrowly.

Three verdicts name how a change meets a consumer:

- **Compatible.** Every ordinary consumer use that compiled against the base
  still compiles against the working tree, within the limits this record
  names.
- **Breaking.** The change takes away a use the base offered, such as a member
  to read, a field to write, or a call with the old arguments, and no position
  of the consumer keeps that use valid.
- **Context-dependent, or not locally provable.** The change is harmless in one
  direction of use and breaks another, or it can change name or overload
  resolution in consumer scope. The answer depends on consumer code or
  language semantics klin does not model.

klin passes only a compatible change that a relaxation below names. It fails
every breaking and every context-dependent change, and every compatible change
no relaxation names yet.

The policy is **conventional source compatibility**. It is not a proof that no
consumer program can break. A TypeScript consumer can reflect the exact key
set of a type, and a Rust addition can make a name ambiguous in a consumer's
scope. Each relaxation names its known limits.

## The decision

### One rule for every language

`public_api` compares two measured contracts of one item with one algorithm,
and it holds no branch for a language:

1. Equal rendered contracts pass.
2. Where either side is not structured, a difference fails.
3. A changed header fails.
4. A part both sides hold with different text fails.
5. A part only the base holds fails.
6. A part only the working tree holds passes when it is omittable and the
   **base** accepts omittable parts. Otherwise it fails.

Every difference must pass for the item to pass. The base decides what it
accepts, so a working tree cannot make itself permissive in the same change
that adds to it.

### The structured contract

A language adapter may give a measured contract structure:

- a **header**, the canonical text that belongs to the item as a whole, such as
  type parameters, heritage or a `where` clause;
- its **parts**, the item's immediate children that a consumer names one by
  one, each with an identity (a name or a position), its canonical text, and
  whether it is **omittable**: a consumer that constructs, implements or calls
  the item may leave it out;
- whether the item **accepts omittable parts**.

The rendered contract is written from the header and the parts, so the two can
never disagree. The adapter decides every one of these facts from its own
parse. The judge never learns why a part is omittable or why an item accepts
one. A contract without structure is compared as a whole, as ADR 0044 did.

### Invariants

- **Lossless.** Where a contract is structured, its header and parts hold the
  whole contract. A member that no relaxation concerns is still a part, or the
  item is not structured.
- **Visible.** A fact whose own change can fail an item appears in the
  rendered contract, so the `was` and `now` lines always differ where klin
  fails an item.
- **Immediate parts only.** A field inside a variant and a parameter inside a
  method stay inside the text of their part, so an addition the outer item
  accepts can never admit a change nested inside an existing part.
- **One declaration.** An item is structured only where exactly one
  declaration makes it. An overload set, a declaration merge and a Rust item
  declared twice under `cfg` are compared as a whole.
- **Unique identities.** An item whose parts do not each have a unique identity
  is not structured.
- **Order where it matters.** The rendered contract keeps the source order of
  signatures inside one file, because overload resolution follows it. Groups
  from different files are ordered by their text, so a renamed file never
  changes a contract (ADR 0001).
- **Missing facts are strict.** A construct the adapter cannot decompose
  safely has no structure.

### The one relaxation: an optional property added to a TypeScript shape

A TypeScript item accepts omittable parts when all of these hold in the base:

- it is an exported interface, or a type alias whose whole right-hand side is
  one object type;
- exactly one declaration makes it;
- it has at least one member, and every member has a unique name.

Each member is a part under its name. A property signature marked `?` is
omittable. Every other member, a method or an optional method included, is
not. An interface's `extends` clause and type parameters are its header.

Example: `interface Point { lat: number; lon: number }` gaining
`height?: number` passes. A required `height: number`, a removed `lon`, a
changed `lat: string` and an added `readonly` each fail.

The base must hold a member because an empty interface is not a weak type and
an interface of optional members is: against a base `interface PluginOptions {}`
a consumer's `use({ name: "x", level: 2 })` compiles, and against
`{ debug?: boolean }` it fails (TS2559).

Known limits of this relaxation:

- a consumer that depends on the exact key set, such as
  `Record<keyof Point, string>`, or a mapped or conditional type over it;
- `in` narrowing over a union: with `type Shape = Circle | Square`, adding
  `radius?: number` to `Square` makes `if ("radius" in s) s.radius * 2` fail
  (TS18048);
- an intersection, or a consumer type that already has a member of the same
  name with another type, including one added by the consumer's own
  declaration merge or module augmentation;
- `exactOptionalPropertyTypes`, under which a consumer's own
  `height?: number | undefined` no longer matches (TS2375).

A class, a call or construct signature, an index signature and every Rust
item have no relaxation yet.

### The strict contract is complete

A strict comparison fails only what it can see, so the canonical contract
shows every fact whose change breaks a consumer:

- a Rust type or variant with a directly written `#[non_exhaustive]` renders
  it;
- a Rust struct with a private named field renders `..` in its field list,
  whether or not the field sits under `#[cfg]`;
- a Rust trait method with a default body renders `{ .. }` where one without
  renders `;`;
- a TypeScript overload set keeps its source order inside one file, and an
  implementation signature that follows overload signatures leaves the set,
  because a consumer never calls it;
- a TypeScript parameter with a default carries the optional marker when no
  required parameter follows it, because a caller may then omit it. The
  initializer stays out of the contract.

An attribute written through `#[cfg_attr(...)]` stays out of the contract,
which is a known limit.

So `#[non_exhaustive]` added to a type or a variant, a private field added to
a struct whose fields were all public, a default body removed from a trait
method, and a change that only reorders overloads each fail. Each passes today
without a finding.

### Changes that stay strict

These changes fail. The table records their verdicts, so a later relaxation
starts from them and not from nothing. "Deferred" marks a change that is
compatible by convention but has no evidence behind a relaxation.

| Change | Verdict | Consumer example or rationale |
|---|---|---|
| TS required member added | context-dependent | A reader is unaffected. `const p: Point = { lat, lon }` fails. |
| TS member removed | breaking | `p.lon` fails. |
| TS member type changed | context-dependent | Widened, `p.lat.toFixed()` fails. Narrowed, `{ lat: "1", lon }` fails. klin infers neither direction. |
| TS `readonly` added | breaking | `p.lat = 2` fails. |
| TS `readonly` removed | compatible, deferred | A consumer that augments the interface with the old modifier can fail. |
| TS optional or defaulted parameter added at the end | compatible, deferred | `[1, 2].map(f)` fails when the new parameter's type does not accept the index. An implementer with its own extra parameter fails (TS2416). |
| TS required parameter made optional | compatible, deferred | A consumer's function assigned to a call signature fails under `strictFunctionTypes` (TS2322). |
| TS parameter or return type changed | context-dependent | Callers and implementers see opposite directions. |
| TS overload added or reordered | context-dependent | Resolution takes the first signature that matches, so `const n: number = parse("1")` can fail. |
| Rust inherent receiver `&mut self` to `&self` | context-dependent | With a trait `Ext::get(&self)` in scope, `t.get()` resolves to the inherent method instead. |
| Rust inherent receiver `&self` to `&mut self` | breaking | `let t = T::new(); t.get()` fails (E0596). |
| Rust trait method receiver changed | breaking | Every implementor fails (E0053). |
| Rust public field added to a struct whose fields were all public | breaking | `S { a, b }` fails (E0063). |
| Rust public field added where the base had a private field or `#[non_exhaustive]` | compatible, deferred | A new `pub len` can shadow a field reached through `Deref`. A private field under `#[cfg]` is absent in some builds. |
| Rust variant added to an exhaustive enum | breaking | A `match` without a wildcard arm fails (E0004). |
| Rust variant added to a `#[non_exhaustive]` enum | compatible, deferred | A variant with data added to a unit-only enum breaks `E::B as u8` (E0605). |
| Rust `#[non_exhaustive]` added to a type or a variant | breaking | A literal and a pattern without `..` fail (E0639, E0638). |
| Rust private field added to a struct whose fields were all public | breaking | `S { a, b }` fails. |
| Rust trait method added with a default body | context-dependent | A consumer with another trait's `name` in scope gets E0034. |
| Rust trait method added without a default body | breaking | Every implementor fails (E0046). |
| Rust default body removed from a trait method | breaking | An implementor that relied on it fails (E0046). |
| Rust function parameter added | breaking | Every call fails (E0061). |

### Adding a relaxation

A later record adds a relaxation when all of these hold:

- a benchmark or a real repository shows a recurring false positive of
  ordinary, intended API growth;
- a fact of the adapter's own parse tells the safe case apart;
- the contract stays lossless, and every nearby case stays strict;
- an adversarial review names the limits;
- CLI tests pin the new pass and the nearby failures.

Such a record changes an adapter's facts. The judge normally stays as it is.

### Adding a language

A language climbs a ladder, one rung at a time, and each check uses the
highest rung the language reaches:

| Rung | The language provides | What it enables |
|---|---|---|
| 0 | a grammar and its function node kinds | complexity, patterns |
| 1 | a structural adapter: declarations, visibility, imports | dead symbols |
| 2 | a module resolver and a surface | layering, and `public-api` for added and removed items |
| 3 | a complete canonical contract | `public-api` for a changed contract |
| 4 | structure for one construct | a relaxation for that construct |

Rung 4 is held per construct, not per language: TypeScript reaches it for
interfaces and object type aliases alone. The registries each layer already
keeps by `LanguageId` are the seams, and no trait layer is added. A rung is
proved by CLI scenarios every language supplies fixtures for: at rung 2 a
removed item fails and an added one passes; at rung 3 a changed contract
fails and a move behind an unchanged identity passes; at rung 4 the
relaxation passes and a required addition, a changed part, a removed part, a
second change beside a compatible one and a merged declaration each fail.

### What klin does not build

- a compiler or a type checker;
- a general subtype engine, or any widening or narrowing inference;
- a consumer scan or any analysis of consumer code;
- a second parser;
- a layer that reparses canonical strings or diffs them by a heuristic.

Canonical strings, whole or of one part, are compared for equality only. The
persisted structural facts change, and so do some rendered contracts, so the
structural-cache epoch rises once for both, and round-trip tests pin the new
facts.

## Accepted entries

The shared lifecycle of an accepted entry does not change. A public-api change
that needed an accepted entry and now passes can leave that entry unmatched.
It follows the stale-entry behavior every gate has: klin reports it as
unmatched, and `--strict` still requires the person to remove it. There is no
public-api migration state. klin deletes no entry, and it does not keep a
compatible finding alive only to consume an old entry.

## The turn-end message

The Regression identity and counting of spec 11.5 do not change to make a
grouped public-api removal read as fewer Regressions.

When every Regression still open in the turn is a public-api finding, the
turn-end `systemMessage` names the problem, not the raw count:

> Public API compatibility breaks still need your attention. `klin stats --turn` shows them.

In every other case the existing counted wording stays, and `klin stats
--turn` keeps its counting. The grouped text report of #304 is presentation
only. JSON and journal identities and accepted entries stay per finding.

## Rejected alternatives

- **A table of compatible changes per language.** The earlier draft of this
  record. Every row leaked under review, and the cost grows with languages
  times rows.
- **Ask once for a context-dependent change**, as ADR 0031 does for a deleted
  test. In b647fbfa8993 the agent made `height` optional to satisfy the gate.
  That change is context-dependent, so asking would let the same shortcut end
  the turn.
- **Reserved policy values**, such as "accept any part" or "accept at the end
  only". A value arrives with the relaxation that needs it.

## Consequences

- #304 carries out this record. SPEC 8.2 states the rule, the structured
  contract, the relaxation and the complete strict contract, and it no longer
  says that the canonical contract drops every attribute.
- SPEC 9.5 says the turn-end line and the report of 11.5 use the same words
  and the same counting rule. The public-api-only line keeps the counting and
  changes the words, so #304 amends 9.5 to name that exception.
- Four breaks that pass today fail: `#[non_exhaustive]` added, a private field
  added to a struct whose fields were all public, a default body removed, and
  overloads reordered.
- A relaxation can pass a change that breaks a reflective or
  resolution-sensitive consumer. Its limits name each known case.
- A new language adds value from rung 2 on, without any compatibility rule.
