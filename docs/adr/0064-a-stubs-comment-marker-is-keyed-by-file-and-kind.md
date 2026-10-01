# A `stubs` comment marker is keyed by its file and kind

> Amends spec 8.2 and 8.2.1. Spec 4.4 lets a check define its own identity
> where no declaration line exists, and this is that case, so ADR 0008 stands.

`stubs` keyed every site by its file plus the trimmed text of its line. A
comment marker is a `TODO`, `FIXME`, `XXX` or `HACK` in a comment. Any edit
to the comment changed its key, so the base held nothing at the new key and
the marker read as new. The replay of #343 (`docs/false-alarms-2026-09-29.md`)
labeled R035 not appropriate: a typo fix in an existing `FIXME` failed
`stubs`. An agent drafted that label and agents reviewed it.

## The decision

**Every comment marker in one file is one site, keyed by the file and the
row kind, `comment marker`, and the site ratchets on `count`.** The count is
the number of marker matches in the file.

- A typo fix inside a marker, or a change from `TODO` to `FIXME`, holds the
  count and passes.
- A new marker raises the count and fails.
- A failure names the marker lines whose text the base file lacks. Each base
  line holds one match with the same text, so a copied marker line is named
  too. The site's line is the first named line, and its values carry all of
  them under `new_lines`.
- A marker moved within a file is held. A marker moved to another file
  raises that file's count, as before.
- An accepted entry for a marker names `comment marker` as its `text`.

Nothing else changes. The code stubs, such as `todo!()`,
`unimplemented!()`, `raise NotImplementedError` and the "not implemented"
panics and throws, keep the file plus line text key, because an edit to such
a line is an edit to code. The body shapes keep their declaration line.
`escapes`, which shares the engine, keeps its key. `stubs` still refuses the
`skip_test_idioms` key.

## Known costs

- A rewording of an existing marker passes, and so does a deletion of one
  marker with a new one added in the same file. The count cannot see either.
- A failure names a file and a count. It may name more than one line when
  several marker lines changed together, and only one of them may be the
  marker that raised the count. The site's line is the first named line, so
  it may point at an edited marker and not at the new one.
- A line that holds a marker and a code stub, such as `todo!() // TODO`, is
  now two sites.
- An accepted entry that keyed a marker by its line text matches nothing,
  and the marker fails as new until a person rewrites the entry. When that
  line also holds a code stub or a body shape, the entry keeps matching the
  site of that code stub or body shape, which now counts it alone. The entry
  holds that site, `--strict` does not name it as stale, and the marker
  fails as new unless the base holds it. The entry may allow a higher count
  than that site holds, and only a person who reads the config sees that.
