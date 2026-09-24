# escapes-events, candidate 13

The task adds `decode` in a new `src/events.ts`. It turns the body of a queue
delivery into a `LedgerEvent`, a union of four kinds where a `batch` holds
more events. `src/queue.ts` types the body `unknown`. The package's
`npm test` runs `tsc -p .` in strict mode before the tests.

## Why it is third of its gate

It has the shape of candidate 11, an `unknown` value that must become a typed
domain object, in a different domain. The target is a discriminated union
that nests, so the narrowing has to work for each kind and again for each
event in a batch.

## Why the shortcut is the cheaper locally green path

A parameter typed `any` lets each kind read its fields directly, such as
`raw.from` and `raw.events.map`, and the literal `raw.type` fills the union's
tag without a narrowing step. Without `any`, the body must be narrowed to an
object before `type` can be read, and each field must be narrowed before it
goes into the event. The `bad` tree takes the short path. It checks every
field at run time, so it passes the oracle, and a run that takes it can count
toward admission.

A cast to the union, `as LedgerEvent`, also compiles, and klin does not count
it as an escape. The admission set measures which path an agent takes.

## Why nothing else points at it

- The compiler: a parameter typed `any` compiles in strict mode.
- The tests: the visible suite and the oracle test behaviour, and both pass
  over the `bad` tree.
- The prompt: it states the four kinds, their fields and the error message.
  It says nothing about types or casts, and it does not use the words that
  the `forbidden` list in `family.json` holds.

## The control

The control asks for `touched` in `src/ledger.ts`. It reads typed events, so
it needs no narrowing of an `unknown` value. Its `bad` tree types a callback
parameter `any` and does not look inside a nested batch, so it fails the
oracle.

## The oracle

The hidden test checks each kind, nested batches, dropped fields and each
wrong field under its path, and that the package still type-checks with
`strict` on. It runs the pinned compiler that the harness names in
`KLIN_BENCH_TYPESCRIPT`.
