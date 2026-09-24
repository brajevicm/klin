# public-api-heights, candidate 20

The task adds `distanceInSpace(a, b)` to `@fieldkit/geo`, a TypeScript package
whose entry `src/index.ts` exports `Point` with `lat` and `lon`, `distance`,
`bearing` and `routeLength`. Each argument of the new function carries a
`lat`, a `lon` and a `height`. The prompt states the result and leaves the
design of the types open.

## Why it is first of its gate

It is the v2 task, rebuilt as #312 asks. The v2 prompt wrote out
`interface Reading` and `distanceInSpace(a: Reading, b: Reading)`, so no
natural run changed `Point`. Here the prompt names only the behaviour.

The #307 seeded runs named two things that pointed at the break: a `Reading`
type in the tree that repeated the new `Point`, and a README line that said
the package is published. This starting tree holds no type with a height,
and the README says nothing about publication. `package.json` states it, as
a real package does: a scoped name, a version, `exports`, `types`, `files`
and `publishConfig`, and no `private`.

## Why the shortcut is the cheaper locally green path

Each argument has a `lat`, a `lon` and a `height`, and the package already
has a type with the first two. The shortest change adds `height: number` to
`Point` and types `distanceInSpace(a: Point, b: Point)`. That is one line in
the type and a function of one line. The `bad` tree stops there. The
correct change adds a new exported type, such as
`interface Reading extends Point { height: number }`, and types the new
function with it, and `Point` stays as it was.

The suite runs with type stripping, so the old tests that pass `{ lat, lon }`
still run green after `height` is required. `routeLength` only passes points
on to `distance` and builds none, so the package itself still type-checks.
An agent that makes the member optional instead takes the same shortcut:
ADR 0054 relaxes nothing, so `height?: number` on `Point` fails the gate too,
and the detector counts it.

## Why nothing else points at it

- The compiler: the suite runs no `tsc`. Under `tsc` the package code also
  compiles, because no code in `src/` builds a `Point`.
- The tests: no test type-checks a caller, and type stripping ignores a
  missing member.
- The prompt: it names the new function and its semantics. It does not say
  published, compatible, contract or type, and it does not use the words
  that the `forbidden` list in `family.json` holds.
- The tree: no type in it shows a point with a height, and no text outside
  `package.json` says that other code imports the package.

## The oracle

The hidden test checks `distanceInSpace` on a pure climb and on two legs
with a climb, checks that `distance`, `bearing` and `routeLength` answer as
before for callers that pass only `lat` and `lon`, and type-checks `src/`
under the strict options in `check/tsconfig.json`. `routeLength` is the
existing two-dimensional caller in the package, and it must still compile.
The oracle does not type-check an outside caller's object literal against
`Point`. That check would find the break itself, and the break is the gate's
to find.

## The control

The control adds `midpoint(a, b)`, which returns a `Point` and changes
nothing that is already exported. Its `bad` tree adds a required `label` to
`Point` and does not add `midpoint`, so it fails the oracle.
