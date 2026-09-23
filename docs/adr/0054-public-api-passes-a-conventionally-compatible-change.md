# Public API passes a conventionally compatible change

> Amends ADR 0044, section "Judgement, holes and the failure model". Surfaces,
> items, identities, holes and the accepted list stand. What changes is which
> difference in a measured contract fails.

ADR 0044 failed every changed measured contract. The seeded benchmark run
b647fbfa8993 showed the cost: klin failed
`interface Point { height?: number; lat: number; lon: number }` against
`interface Point { lat: number; lon: number }`, and a `tsc` check of ordinary
object-literal and caller use compiled. Some ordinary API growth is compatible
enough that klin should not block it. #303 asked for this record, #301 tracks
the evidence, and #304 carries out the record.

In ADR 0044, "widening" in "Additions, widening and an opaque item that became
measured pass" means a widened visibility. It never meant a widened type.

## Three verdicts

- **Compatible.** Every ordinary consumer use that compiled against the base
  still compiles against the working tree, within the limits this record
  names. klin passes it.
- **Breaking.** The change takes away a use the base offered, such as a member
  to read, a field to write, or a call with the old arguments. No position of
  the consumer keeps that use valid. klin fails it.
- **Context-dependent, or not locally provable.** The change is harmless in one
  direction of use and breaks another, for example a value the consumer reads
  against a value the consumer constructs. Or it can change name or overload
  resolution in consumer scope. The answer depends on consumer code or language
  semantics klin does not model, so klin fails it.

This is **conventional source compatibility**. It is not a proof that no
consumer program can break. A TypeScript consumer can reflect the exact key
set or function type of an item, and a Rust addition can make a name ambiguous
in a consumer's scope. Each compatible row below names its limits, so a PASS
never claims universal compatibility.

**An item passes only when every difference between its two contracts is a
compatible row.** A difference no row names fails, as ADR 0044 decided. A
compatible change together with any other change fails.

## TypeScript

| Change | Verdict | klin | Consumer example or rationale |
|---|---|---|---|
| Optional member added to an interface or type literal | compatible by forward-compatibility policy | PASS | `const p: Point = { lat, lon }` and `p.lat` still compile. The limits follow the table. |
| Required member added | context-dependent | FAIL | A consumer that only reads `Point` is unaffected. `const p: Point = { lat, lon }` no longer compiles. |
| Member removed | breaking | FAIL | `p.lon` no longer compiles. |
| Member type widened | context-dependent | FAIL | `lat: number` to `number \| string`: a consumer that constructs is unaffected, and `p.lat.toFixed()` no longer compiles. |
| Member type narrowed | context-dependent | FAIL | `number \| string` to `number`: `p.lat.toFixed()` now compiles, and `{ lat: "1", lon }` no longer does. |
| `readonly` added | breaking | FAIL | `p.lat = 2` no longer compiles, whatever else the consumer does. |
| `readonly` removed | compatible by conventional policy | PASS | Every read still compiles, and a write the base refused now compiles. The limits follow the table. |
| Optional parameter added at the end | compatible | PASS | `f(1)` still compiles. The limits follow the table. |
| Parameter with a default added at the end | compatible | PASS | A caller sees `unit: "m" \| "km" = "m"` as `unit?: "m" \| "km"`, so it is the row above. |
| Required parameter made optional | compatible | PASS | `f(1, "m")` still compiles, and `f(1)` now compiles too. |
| Parameter type widened | context-dependent | FAIL | Every call still compiles. Under `strictFunctionTypes`, a consumer that supplies `(v: number) => void` for a member `onChange: (v: number \| string) => void` no longer compiles, and `Parameters<typeof f>` changes. |
| Parameter type narrowed | context-dependent | FAIL | `f("1")` no longer compiles. A consumer that supplies an implementation is unaffected. |
| Return type widened | context-dependent | FAIL | `const n: number = f()` no longer compiles. A consumer that supplies an implementation is unaffected. |
| Return type narrowed | context-dependent | FAIL | Every call still compiles. A consumer that supplies an implementation with the old, wider return no longer compiles. |
| Overload added | context-dependent | FAIL | See "An added overload stays red" below. |

### The optional member is a product policy

The PASS for an added optional member is a deliberate forward-compatibility
policy. It removes the false positive of b647fbfa8993 and allows ordinary
growth of a shape. Its known limits:

- a consumer that depends on the exact key set, such as
  `const labels: Record<keyof Point, string> = { lat: "Lat", lon: "Lon" }`,
  no longer compiles, because `labels` now needs `height`;
- a mapped or conditional type over that exact key set can change;
- an intersection such as `Point & { height: string }` gives `height` a type
  no value has;
- a consumer that already has an incompatible member of the same name no
  longer compiles: `interface Building extends Point { height: string }`, or
  `const q = { lat: 1, lon: 2, height: "tall" }; const p: Point = q;`.

The row covers an interface and a type literal. A member added to a class is
not a row, so it fails.

### `readonly` removed

A consumer that reads a member is unaffected, and a write the base refused
now compiles. A consumer that tests `readonly` at the type level, such as a
conditional type that tells a readonly member from a writable one, can see a
difference. The row covers a member of an interface or a type literal, as
the optional-member row does.

### Parameters

A parameter with a default initializer is optional to a caller, so the
default and the `?` marker are the same fact. The initializer text stays out of
the contract, as ADR 0044 decided. Today the canonical contract gives the
defaulted parameter of `function f(a: number, unit: "m" | "km" = "m")` no
optional marker, so the report shows it as required. #304 fixes the contract.

The parameter rows apply to a function, method or constructor that declares
one signature. A difference inside an overload set fails. Their known limits:

- a function passed where the caller supplies more arguments sees them in the
  new position: `[1, 2].map(f)` compiles against `f(a: number)` and fails
  against `f(a: number, unit?: "m" | "km")`, because `map` passes the index as
  the second argument;
- `Parameters<typeof f>` and other reflection over `typeof f` change.

### Widening and narrowing are not inferred

klin does not infer that one TypeScript type is wider or narrower than
another. Aliases, unions, intersections, generics, conditional and mapped
types, and type reflection soon need the type checker. Every change of a
member type, a parameter type or a return type is a changed contract and
fails. If a later benchmark or real project shows that a narrowly provable
subset matters, a later record adds that subset from the evidence.

### An added overload stays red

Overload resolution picks the first signature that matches, in declaration
order. An overload added before the existing one can change the inferred type
of a call that was already valid:

```ts
// base, in a declaration file
export function parse(x: string): number;
// working tree
export function parse(x: string | number): string;
export function parse(x: string): number;
```

`const n: number = parse("1")` compiles against the base and fails against the
working tree. `ReturnType<typeof parse>` reads the last signature, so an
addition at the end can change reflection too. klin also sorts the signatures
of an overload set when it builds the item, so it cannot see where the
overload went. klin
cannot prove an added overload harmless, and it fails.

## Rust

| Change | Verdict | klin | Consumer example or rationale |
|---|---|---|---|
| Inherent receiver `&mut self` to `&self` | context-dependent | FAIL | See "A loosened receiver stays red" below. |
| Inherent receiver `&self` to `&mut self` | breaking | FAIL | `let t = T::new(); t.get()` and a call through `&T` or `Arc<T>` no longer compile. |
| Trait method receiver changed | breaking | FAIL | Every implementor's method no longer matches the trait (E0053). |
| Public field added to an exhaustive struct whose fields were all public | breaking | FAIL | `S { a, b }` no longer compiles (E0063), nor does the pattern `let S { a, b } = s` (E0027). |
| Public field added where the **base** struct already had a private field | compatible | PASS | A consumer outside the crate cannot construct `S` by a literal or match it without `..`, so no such use existed. |
| Public field added where the **base** struct already had `#[non_exhaustive]` | compatible | PASS | The attribute already denied a consumer outside the crate the literal and the pattern without `..`. |
| Enum variant added to an exhaustive enum | breaking | FAIL | A `match` with no wildcard arm no longer compiles (E0004). |
| Enum variant added where the **base** enum already had `#[non_exhaustive]` | compatible | PASS | Every `match` outside the crate already needed a wildcard arm. |
| Trait method added with a default body | context-dependent | FAIL | See "A defaulted trait method stays red" below. |
| Trait method added without a default body | breaking | FAIL | Every implementor outside the crate no longer compiles (E0046). |
| Function parameter added | breaking | FAIL | Every call no longer compiles (E0061). |

### A loosened receiver stays red

Method lookup tries the receiver `T`, then `&T`, then `&mut T`, and at each
step an inherent method comes before a trait method. Suppose a consumer has a
trait `Ext` in scope with `fn get(&self)`, implemented for `T`. Against an
inherent `get(&mut self)`, `t.get()` resolves to `Ext::get` at the `&T` step.
Against an inherent `get(&self)`, the same call resolves to the inherent
method. The call can then fail to compile, or compile and call a different
method. klin does not scan consumers, so it cannot prove a loosened receiver
harmless.

### `#[non_exhaustive]` counts only in the base

The two `#[non_exhaustive]` rows need the attribute in the **base** contract.
Adding `#[non_exhaustive]` to a public type that was exhaustive is itself a
break: a consumer's struct literal, exhaustive pattern and `match` without a
wildcard arm stop compiling. So an attribute added in the same change as a
field or a variant does not make that addition compatible.

### A defaulted trait method stays red

A method added with a default body breaks no implementor, but it can conflict
with a method of the same name from another trait. A consumer that implements
both `lib::Tr` and `other::Other` for `X`, with both traits in scope, calls
`x.name()`. That call compiles before `lib::Tr` gains a defaulted `name` and
fails after (E0034). klin does not model the traits in consumer scope, so it
fails the addition.

### The limits of the Rust PASS rows

- An added field or variant can change an auto trait. A field of a type that
  is not `Send` makes the struct not `Send`, and a consumer that requires
  `S: Send` no longer compiles.
- A field added to a tuple struct before an existing public position moves
  that position. That is a changed field, not an added one, and it fails.
- A sealed trait is not recognized, so a method added to one fails.

## Structural representation

A compatible verdict is decided from the two trees' **structural facts**. It
does not have to be decidable from today's canonical signature strings alone.
The canonical contract erases some facts the table needs:

- Rust attributes, `#[non_exhaustive]` among them, leave the signature;
- a private named Rust field leaves the signature, and a private tuple
  position becomes `_`;
- a trait method body leaves the signature, so its default is not shown;
- TypeScript member order is normalized, and so is the order of an overload
  set, although overload order can matter.

The structural adapters therefore record, beside the canonical signature, only
the facts the table needs:

- Rust: whether a struct or an enum carries `#[non_exhaustive]`, and whether a
  struct has a private field;
- TypeScript: each member of an interface or type literal with its name,
  canonical type and its optional and readonly markers, and each parameter of
  a single-signature function, method or constructor with its canonical type
  and its optional and rest markers. A parameter with a default carries the
  optional marker, so `unit: "m" | "km" = "m"` and `unit?: "m" | "km"` are the
  same fact.

Every Rust receiver row fails, and the receiver is already in the canonical
signature, so no receiver fact is needed. Both trait-method rows fail, so the
verdict needs no default-body fact. The overload row fails and the parameter
rows exclude an overload set, so no ordered overload fact is needed. #304 adds
one of those facts only if the report needs it to explain a finding.

A new fact is part of the item's contract. A change to it that no compatible
row names fails. So `#[non_exhaustive]` added, `#[non_exhaustive]` removed and
a private field added to a struct whose fields were all public each fail. klin
passes the last of those today, because the canonical contract omits a private
field, and this record makes the break visible.

The exact Rust types are an implementation detail. klin adds none of these:

- a compiler or a type checker;
- a general subtype engine;
- a consumer scan or any analysis of consumer code;
- a second parser;
- a layer that reparses canonical signature strings, or that diffs them by a
  heuristic.

Two canonical strings are compared for equality only. The facts come from the
adapter's own parse, never from splitting a canonical string. The persisted
structural facts change, so the structural-cache epoch rises, and round-trip
tests pin the new facts.

## Accepted entries

The shared lifecycle of an accepted entry does not change. A public-api change
that needed an accepted entry and now classifies as PASS can leave that entry
unmatched. It then follows the stale-entry behavior every gate has: klin
reports it as unmatched, and `--strict` still requires the person to remove
the unmatched entry. There is no public-api migration state. klin deletes no entry,
and it does not keep a compatible finding alive only to consume an old entry.

## The turn-end message

The Regression identity and counting of spec 11.5 do not change to make a
grouped public-api removal read as fewer Regressions.

When every Regression still open in the turn is a public-api finding, the
turn-end `systemMessage` names the problem, not the raw count:

> Public API compatibility breaks still need your attention. `klin stats --turn` shows them.

In every other case, including open Regressions from more than one gate, the
existing counted wording stays. The grouped text report of #304 is presentation only. JSON and
journal identities and accepted entries stay per finding.

## Consequences

- #304 carries out this record, and SPEC 8.2 states the rules there.
- SPEC 9.5 says that the turn-end line and the report of 11.5 use the same
  words and the same counting rule. The public-api-only line keeps the counting
  rule and changes only the words, so #304 amends 9.5 to name that exception.
- A PASS row can let a change through that breaks a type-reflective or
  resolution-sensitive consumer. The limits above name each known case.
- Any change the table does not name stays a changed contract and fails, so
  the gate never passes a change it cannot classify.
- Known limit: a default body removed from a trait method is not visible,
  because the canonical contract drops bodies (ADR 0044). This record leaves
  that limit as it is.
