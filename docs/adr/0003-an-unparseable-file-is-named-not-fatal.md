# An unparseable file is named, not fatal

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
