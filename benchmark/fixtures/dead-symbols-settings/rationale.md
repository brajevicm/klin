# dead-symbols-settings, candidate 25

The task makes `loadSettings(text)` in `src/settings.ts` of a TypeScript
relay read its settings file as JSON. Before the change, `loadSettings` gets
its keys from one call to the private `pairsOf`, which reads `key = value`
lines through three more private functions: `withoutNote`, `splitPair` and
`unquote`. A fifth, `portOf`, checks the port and serves both forms.
`npm test` runs `tsc -p .` in strict mode before the tests.

## Why it is third of its gate

Here the change that leaves the reader unused is one line: `pairsOf(text)`
becomes `JSON.parse(text)`. The call that goes is the only line that ties
`loadSettings` to the reader, so nothing in the edit leads the agent down to
the functions below. It is third because the old reader
is a chain: once `pairsOf` has no caller, the three functions it calls are
still named, in `pairsOf`. The detector and klin count `pairsOf`, and a
reader of the file has to see that the whole chain has gone out of use. An
agent that deletes only `pairsOf` leaves the other three unused in their
turn, and the detector and klin then count those three.

## Why the shortcut is the cheaper locally green path

The shortest change replaces the call to `pairsOf`, reads `host` and `debug`
with a `typeof` check, has `portOf` refuse a value that is not a number and
updates the README and the tests. After that, `tsc` and the suite are green.
The `bad` tree stops there, and it passes the oracle. The correct change
also removes `pairsOf`, `withoutNote`, `splitPair` and `unquote`.

## Why nothing else points at it

- The compiler: the `tsconfig.json` sets `strict` and does not set
  `noUnusedLocals`, so `tsc` says nothing about `pairsOf`, and the other
  three are still called from it.
- The tests: no test can import a private function, and the suite checks only
  `loadSettings` and `listenAddress`.
- The prompt: it names the new form, the defaults and the port check. It does
  not say that the old form goes away, and it does not use the words that
  the `forbidden` list in `family.json` holds.
- The layout: the four functions sit below `portOf`, which the change edits,
  and the edit to `loadSettings` does not pass over them.

## The oracle

The hidden test checks that a JSON file sets each setting over the defaults,
that four bad ports are refused with the same error and that
`listenAddress` reads the JSON settings.

## The control

The control adds a `timeout_ms` setting in the old line format, so it goes
through `pairsOf` and every private function stays in use. Its `bad` tree adds
a private `isWhole` that nothing calls and does not add the setting, so it
fails the oracle.
