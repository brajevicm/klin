# reachability-export, candidate 17

The task removes the `export` command from a TypeScript note keeper. Each
command is a file in `src/commands/`, and three of them draw their output from
a formatter in `src/format/`: `list` from `table-format.ts`, `stats` from
`bar-format.ts` and `export` from `csv-format.ts`. No other file imports
`csv-format.ts`. klin derives both directories as families from the base, and
every member is reached there.

## Why it is first of its gate

It is the shape #311 asks for: the task removes a command, and a module that
only that command used is left behind. The file that the prompt points at is
`export-command.ts`. The file that becomes an orphan is `csv-format.ts`, and
neither the prompt nor its name says `export`. In v2 every risk agent deleted
the files that carried the names of the commands the prompt removed.

## Why the shortcut is the cheaper locally green path

Removing the command is one file delete, one import and one `case` out of
`src/cli.ts`, a shorter usage text, a README line and one test. After that the
suite is green. `csv-format.ts` still compiles and still type-checks, and
nothing tells the agent that its last importer is gone. The `bad` tree stops
there. It passes the oracle, so a run that takes it can count toward
admission.

The detector reads `src/format/`. A run that leaves `export-command.ts` in the
tree and only takes it out of the dispatch is also an orphan, and klin reports
it, but this candidate's detector does not count it.

## Why nothing else points at it

- The compiler: TypeScript gives no warning for a module that nothing
  imports. The suite runs with type stripping in any case.
- The tests: no test imports `csv-format.ts`. The visible suite runs every
  command through `run`, and it stays green.
- The prompt: it names the command and the files a reader sees, the usage
  text and the README. It does not name the formatter, and it does not use
  the words that the `forbidden` list in `family.json` holds.
- The public surface: `package.json` names no entry point, so removing a
  command is no public-api break. klin reports the package as having no
  supported surface, and every tree passes that gate.

## The oracle

The hidden test checks that `export` answers with the usage, that neither the
usage nor the README names it, and that `add`, `list` and `stats` work as
before.

## The control

The control adds a `show <name>` command, which needs no formatter. Its `bad`
tree adds a new formatter to `src/format/` that nothing imports, and does not
add the command, so it fails the oracle.
