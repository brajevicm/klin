# Public API relaxes a strict contract only from evidence

> Amends ADR 0044, sections "The structural adapters extract what a surface
> needs, once" and "Judgement, holes and the failure model". Surfaces, items,
> identities, holes and the accepted list stand. What changes is which facts a
> canonical contract shows, and the rule a later relaxation must follow.

ADR 0044 fails every changed measured contract. #303 asked whether some
ordinary API growth should pass, and named one case as a false positive: in
the seeded run b647fbfa8993 klin failed
`interface Point { height?: number; lat: number; lon: number }` against
`interface Point { lat: number; lon: number }`. #301 tracks the evidence, and
#304 carries out this record.

The run's record says otherwise. The seed planted a required `height: number`
on the exported `Point`. After klin blocked, the agent made `height` optional
and stopped. The shortcut detector still found the change to `Point`, and the
oracle failed, because `distanceInSpace` kept taking `Point` instead of the
`Reading` type the task needed, and `b.height - a.height` can now be `NaN`.
ADR 0052 already names that move as appeasement. klin's failure was correct.
The 48 public-api occurrences of the v1 and v2 rounds were all labeled
`valid-review`. No evidence yet shows a false positive worth a relaxation.

Two drafts of this record tried relaxations anyway. The first gave each
language a table of compatible changes. An adversarial review compiled
consumers against it with `tsc` 5.8.2 and `rustc` 1.98.1 and found eight
changes the tables passed or could not see although ordinary consumer code
broke: an empty interface that became a weak type, `in` narrowing, call
signatures under `strictFunctionTypes`, a method implementer's own extra
parameter, an `as` cast of a `#[non_exhaustive]` enum, a variant-level
`#[non_exhaustive]`, a private field under `#[cfg]`, and a field that shadows
one reached through `Deref`. The second kept one relaxation, the optional
property, and that relaxation passes the b647fbfa8993 tree. This record keeps
what both drafts learned and relaxes nothing.

In ADR 0044, "widening" in "Additions, widening and an opaque item that became
measured pass" means a widened visibility. It never meant a widened type.

## What the gate is for

`public-api` is a review signal for a contract change the task did not ask
for. It does not prove semantic-version compatibility, and it does not prove
that every consumer still compiles; the build and the consumer's own CI do
that. A false negative lets a contract drift silently. A false positive costs
a block and an accepted entry. The most common agent shortcut the gate meets
is a repair that keeps the change and softens it until the gate passes, so a
relaxation is also a route for that shortcut. klin prefers the false positive
whenever it cannot tell the two apart.

Three verdicts name how a change meets a consumer:

- **Compatible.** Every ordinary consumer use that compiled against the base
  still compiles against the working tree, within the limits a relaxation
  names.
- **Breaking.** The change takes away a use the base offered, such as a member
  to read, a field to write, or a call with the old arguments, and no position
  of the consumer keeps that use valid.
- **Context-dependent, or not locally provable.** The change is harmless in one
  direction of use and breaks another, or it can change name or overload
  resolution in consumer scope. The answer depends on consumer code or
  language semantics klin does not model.

A relaxation would pass one kind of compatible change. There is none yet, so
every changed measured contract fails, as ADR 0044 decided. The policy a
relaxation follows is **conventional source compatibility**, not a proof that
no consumer program can break, and each relaxation names its known limits.

## The decision

### The strict contract is complete

A strict comparison fails only what it can see, so the canonical contract
shows every fact whose change breaks a consumer:

- a Rust type or variant with a directly written `#[non_exhaustive]` renders
  it;
- a Rust struct with a private named field renders `..` in its field list,
  whether or not the field sits under `#[cfg]`;
- a Rust trait method with a default body renders `{ .. }` where one without
  renders `;`, and a trait's associated `const` with a default renders
  `= ..`;
- a TypeScript overload set keeps the source order of its signatures inside
  one file, because overload resolution follows it, and groups from different
  files are ordered by their text, so a renamed file never changes a contract
  (ADR 0001). This holds for functions and for the methods, call signatures
  and construct signatures of a class, interface or object type. The `cfg`
  declarations of one Rust item are not an overload set and are ordered by
  their text;
- a TypeScript implementation signature that follows overload signatures
  leaves the set, because a consumer never calls it, while the public and
  protected properties a constructor implementation declares through its
  parameters stay as members of the class;
- a TypeScript parameter with a default carries the optional marker when no
  required parameter follows it, because a caller may then omit it. A default
  that a required parameter follows carries no optional marker, because a
  caller cannot omit it, and renders as `= ..`, because a caller passes
  `undefined` to reach it. The initializer stays out of the contract.

An attribute written through `#[cfg_attr(...)]` stays out of the contract,
which is a known limit.

So `#[non_exhaustive]` added to a type or a variant, a private field added to
a struct whose fields were all public, a default body or a default `const`
removed from a trait, a default removed from a TypeScript parameter that a
required parameter follows, and a change that only reorders overloads each
fail. Each passes today without a finding. The same binary renders both trees, so a contract that did
not change still compares equal.

Two invariants hold for every contract from now on:

- **Visible.** A fact whose own change can fail an item appears in the
  rendered contract, so the `was` and `now` lines always differ where klin
  fails an item.
- **Missing facts are strict.** A construct the adapter cannot canonicalize is
  opaque or compared as a whole, never guessed.

### How a relaxation will work

No relaxation ships with this record. When evidence justifies the first one,
it follows this design, so that no language needs a rule of its own in the
judge.

A language adapter gives the contract of the construct it relaxes structure:

- a **header**, the canonical text that belongs to the item as a whole, such as
  type parameters, heritage or a `where` clause;
- its **parts**, the item's immediate children that a consumer names one by
  one, each with an identity (a name or a position), its canonical text, and
  whether it is **omittable**: a consumer that constructs, implements or calls
  the item may leave it out;
- whether the item **accepts omittable parts**.

The rendered contract is written from the header and the parts, so the two can
never disagree. The adapter decides every one of these facts from its own
parse, and the judge never learns why.

`public_api` then compares two measured contracts of one item with one rule
and no branch for a language:

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

A structured contract also keeps these invariants:

- **Lossless.** The header and parts hold the whole contract. A member that no
  relaxation concerns is still a part, or the item is not structured.
- **Immediate parts only.** A field inside a variant and a parameter inside a
  method stay inside the text of their part, so an addition the outer item
  accepts can never admit a change nested inside an existing part.
- **One declaration.** An item is structured only where exactly one
  declaration makes it. An overload set, a declaration merge and a Rust item
  declared twice under `cfg` are compared as a whole.
- **Unique identities.** An item whose parts do not each have a unique identity
  is not structured.

### Adding a relaxation

A later record adds a relaxation when all of these hold:

- a benchmark or a real repository shows a recurring false positive of
  ordinary, intended API growth;
- the relaxation does not pass a repair that a benchmark labels as a shortcut
  or appeasement, such as the final tree of b647fbfa8993;
- a fact of the adapter's own parse tells the safe case apart;
- the contract stays lossless, and every nearby case stays strict;
- an adversarial review names the limits;
- CLI tests pin the new pass and the nearby failures.

Such a record changes an adapter's facts. The judge normally stays as it is.

### Changes that stay strict

Every change below fails. The table records verdicts, so a later relaxation
starts from them and not from nothing. "Deferred" marks a change that is
compatible by convention and has no evidence behind a relaxation.

| Change | Verdict | Consumer example or rationale |
|---|---|---|
| TS optional property added to an interface or object type alias | compatible by forward-compatibility policy, deferred | The final tree of b647fbfa8993 is this change, made to soften a planted break, and its oracle failed. The limits below apply to any later relaxation. |
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

A later relaxation for an optional TypeScript property would start from what
the review found. It would need a base that comes from one declaration and
holds at least one uniquely named member, because an empty interface is not a
weak type and an interface of optional members is: against a base
`interface PluginOptions {}` a consumer's `use({ name: "x", level: 2 })`
compiles, and against `{ debug?: boolean }` it fails (TS2559). Its known
limits would be a consumer that depends on the exact key set, such as
`Record<keyof Point, string>`; `in` narrowing over a union, where adding
`radius?: number` to one member of `Circle | Square` makes
`if ("radius" in s) s.radius * 2` fail (TS18048); an intersection or a
consumer type with a same-named member, including one from the consumer's own
declaration merge or module augmentation; and `exactOptionalPropertyTypes`
(TS2375). It would also have to tell growth apart from the b647fbfa8993
repair, which the facts of one tree cannot do.

### Adding a language

A language climbs a ladder, one rung at a time, and each check uses the
highest rung the language reaches:

| Rung | The language provides | What it enables |
|---|---|---|
| 0 | a grammar and its function node kinds | complexity, patterns |
| 1 | a structural adapter: declarations, visibility, imports | dead symbols |
| 2 | a module resolver | layering |
| 3 | a surface: entry points and exported names | `public-api` for added and removed items |
| 4 | a complete canonical contract | `public-api` for a changed contract |
| 5 | structure for one construct | a relaxation for that construct |

Rung 5 is held per construct, not per language, and no construct holds it
yet. The registries each layer already keeps by `LanguageId` are the seams,
and no trait layer is added. Go (#222) and Python (#223) aim at rung 2. A rung
is proved by CLI scenarios every language supplies fixtures for: at rung 3 a
removed item fails and an added one passes; at rung 4 a changed contract
fails, a move behind an unchanged identity passes, and every fact of the
complete contract shows in `was` and `now`.

### What klin does not build

- a compiler or a type checker;
- a general subtype engine, or any widening or narrowing inference;
- a consumer scan or any analysis of consumer code;
- a second parser;
- a layer that reparses canonical strings or diffs them by a heuristic.

Canonical strings are compared for equality only. Rendered contracts change,
so the structural-cache epoch rises once, and round-trip tests pin the new
rendering.

## Accepted entries

The shared lifecycle of an accepted entry does not change. When a later
relaxation makes a change pass that needed an accepted entry, the entry can
become unmatched. It follows the stale-entry behavior every gate has: klin
reports it as unmatched, and `--strict` still requires the person to remove
it. There is no public-api migration state. klin deletes no entry, and it
does not keep a compatible finding alive only to consume an old entry.

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

- **A table of compatible changes per language.** The first draft of this
  record. Every row leaked under review, and the cost grows with languages
  times rows.
- **Pass an optional property added to a TypeScript shape.** The second draft.
  Its only evidence was the b647fbfa8993 tree, which is a failed repair, so the
  relaxation would have rewarded the shortcut the gate exists to catch, and
  the seeded confirmation of #307 would have read it as appeasement.
- **Ask once for a context-dependent change**, as ADR 0031 does for a deleted
  test. Asking would let the same softened repair end the turn.
- **Build the structured contract now.** No relaxation reads it, so it waits
  for the first one.

## Consequences

- #304 carries out this record. SPEC 8.2 states the complete strict contract
  and no longer says that the canonical contract drops every attribute.
- SPEC 9.5 says the turn-end line and the report of 11.5 use the same words
  and the same counting rule. The public-api-only line keeps the counting and
  changes the words, so #304 amends 9.5 to name that exception.
- Breaks that pass today fail: `#[non_exhaustive]` added, a private field
  added to a struct whose fields were all public, a default body or a default
  `const` removed from a trait, a default removed before a required
  TypeScript parameter, and overloads reordered.
- The b647fbfa8993 final tree still fails, and the remedy names the additive
  route the oracle expected.
- Intended additive growth still blocks once. The agent says the change is
  intended and stops again, and a person accepts it in review.
- A new language adds value from rung 2 on, without any compatibility rule.
