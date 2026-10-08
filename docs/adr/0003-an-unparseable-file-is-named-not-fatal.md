# An unparseable file is named, not fatal

> ADR 0021 amends this record. In `--hook` mode an unreadable file is a note
> and does not block. Under `--strict` the exit 2 below stands.
>
> The amendment below (#500) ends the exit 2. Spec 7.2 sorts a file klin
> cannot read into a `measurement-lost` FAIL, an opened gap or a coverage
> note.

A grammar rejects a file more often than it looks. Flow-typed `.js` is ordinary
in a React Native tree and the JavaScript grammar refuses it. A grammar one
release behind its language refuses whatever the language added. detent reads
eight of them, so the case is routine rather than exotic.

The complexity gate used to abort the whole run on the first such file: exit 2,
no findings, nothing about the rest of the tree. One file the grammar could not
read hid every function detent had already measured.

It no longer does. The gate measures every file it can parse, prints those
findings, then names each file it could not parse and the grammar that rejected
it. The run still ends in exit 2, the tool-error code, because a file detent
could not measure is a hole in the ratchet and not debt a baseline holds. This
matches the ladder, where `gate` already reports 2 when one gate errored and
another only failed. Under `--only` the report is restricted the same way the
findings are: a file outside the judged set is out of scope, hole or not.

## The baseline entries of a file that stopped parsing

A file measures no functions once its grammar rejects it, so its baseline
entries would match nothing and read as fixed. Two things follow, and both would
throw away debt a person accepted.

The gate holds those entries out of the judgement. They are neither stale nor
loose, so `--strict` does not report the baseline as looser than the code and
does not tell the reader to rewrite it.

`--write-baseline` refuses to run at all while any file goes unparsed, and names
the files. A baseline written then would be missing whatever those files hold,
and nothing in the file would say so. Update the grammar, or exclude the file,
and then write the baseline.

## Consequences

Exit 2 no longer implies that the output is empty. A caller reading detent's
exit code alone sees no change. A caller reading the text sees the findings it
would otherwise have lost.

A tree holding a file no grammar can read cannot accept new complexity debt
until a person excludes that file. This is the intended pressure: the exclusion
is a reviewed decision, and the entry in `quality.json` is where the next reader
finds out that nothing measures the file.

## Amendment: lost, opened or klin's own limit (#500)

The exit 2 above treated every file klin could not read as one hole. It is
three different things, and spec 7.2 now tells them apart once for the whole
run, against the base.

- A file the base measured and the change made unmeasurable is a
  `measurement-lost` FAIL, exit 1. The reasons are a new error node, a line
  over the source-line ceiling, a manifest or lockfile that no longer parses,
  and a form change: a NUL byte, a symbolic link, or a `binary`, `-diff` or
  `filter` attribute. The agent caused it and can fix it, so it fails and
  blocks the Stop. This closes the route of hiding a finding by making its
  file unmeasurable.
- A file the change made unmeasurable with no clear agent cause, such as a
  new file the grammar rejects, is an `unmeasured` review item, exit 0.
- A file the base could not measure either is a coverage note.

None of the three is exit 2, which is now for errors only (spec 7.3). The
pressure this record wanted stays: the coverage counts and `not_measured`
still name every file nothing measured. A valid construct that klin's grammar
does not read yet is the one false positive, and a person holds that file
with the accepted entry `{"gate": "measurement-lost", "file": PATH}` in a
reviewed commit.
