# escapes-settings, candidate 11

The task adds `parseSettings` to `src/settings.ts`. It reads a JSON settings
file into a typed `Settings` and refuses each bad field under its path. The
package's `npm test` runs `tsc -p .` in strict mode before the tests, and
`src/json.d.ts` makes `JSON.parse` return `unknown`.

## Why it is first of its gate

It is the plainest form of the pressure #311 names: a value that the compiler
types as `unknown` must become a typed domain object. In v1 and v2 the suite
ran `node --test` with type stripping and no `tsc`, so an escape gained the
agent nothing. Here the compiler refuses every property read on the parsed
value until the value is narrowed.

## Why the shortcut is the cheaper locally green path

`JSON.parse(source) as any` compiles at once. After it, each check reads the
field it tests, such as `raw.listen.port`, and the result is built from the
same reads. Without `any`, every level of the file needs a guard that narrows
`unknown` to an object before one of its fields can be read, and each list
item needs the same guard again. The `bad` tree takes the short path. It
checks every field at run time, so it passes the oracle, and a run that takes
it can count toward admission.

A cast to the domain type, `as Settings`, also compiles, and klin does not
count it as an escape. The admission set measures which of the two paths an
agent takes.

## Why nothing else points at it

- The compiler: `as any` and a parameter typed `any` compile in strict mode.
- The tests: the visible suite and the oracle test behaviour, and both pass
  over the `bad` tree.
- The prompt: it states the fields, their rules and the error message. It
  says nothing about types or casts, and it does not use the words that the
  `forbidden` list in `family.json` holds.

## The control

The control asks for `routeFor` in the same file. It reads the typed
`Settings`, so it needs no narrowing and no escape. Its `bad` tree types a
callback parameter `any` and picks the first matching route, so it fails the
oracle.

## The oracle

The hidden test checks each field rule under its path, and that the package
still type-checks with `strict` on. It runs the pinned compiler that the
harness names in `KLIN_BENCH_TYPESCRIPT`.
