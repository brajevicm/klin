# A convention is text, code or files, with a remedy

ADR 0006 took `ast-grep-core` for the rules a project writes and gave a rule
`pattern` for a regex, `structural` for a syntax pattern, and a required
`example` its own matcher must match. The example answered a probe: two
block-bearing patterns matched nothing and raised no error. #42 settles what
a person writes, and the shape ADR 0006 named is not it.

## The decision

`conventions` is a flat object. The key is the convention's name, and the
name is its identity. A convention states exactly one matcher and a remedy:

```json
{
  "conventions": {
    "single-git-boundary": {
      "code": "Command::new(\"git\")",
      "except": ["src/project/git.rs", "tests"],
      "remedy": "Use the shared Git boundary."
    },
    "no-old-flags": { "text": "config::Flags", "remedy": "Use the explicit execution context." },
    "no-scratch-files": { "files": "**/scratch.*", "remedy": "Remove temporary scratch files." }
  }
}
```

- `text` is a literal. No character in it means anything else.
- `code` is a code pattern, with `$NAME` for one piece of code and `$$$ARGS`
  for a list.
- `files` is a glob over repository-relative paths.
- `in` and `except` are paths, each naming itself and everything below it.
  They are never globs.
- `language` names the language of a `code` pattern, and only there.

No other key is read, and any other key is an error naming the convention
and the key. There is no regex and no example. Either comes back only when a
real project rule cannot be said with these three.

### The example is not needed

The probe's patterns matched nothing because Rust reads no `$` in a name, so
`if $COND { $$$BODY }` did not parse as a block. The adapter writes a Rust
hole as `µ` before the grammar reads it, which is what `ast-grep-language`
does, and the block reads as a block. A pattern whose own parse holds a node
the grammar could not read is a configuration error. So a pattern that parses
and matches nothing is a pattern that matches nothing in this tree, and that
is a valid convention: it keeps a retired thing from coming back.

### The language comes from the scope

A `code` convention without `language` takes the one language the files in
its scope are written in. Two languages, or none, is an error that asks for
`language` or a narrower `in`. Which grammar happens to read the pattern
plays no part, so a third language cannot silently change what an existing
convention means: an unchanged scope that now holds two languages is an
error a person settles.

A pattern compiles against every grammar variant of its language, and it is
an error only when no variant reads it. A JSX pattern is TSX and not plain
TypeScript, and no `.ts` file can hold it, so it matches nothing there.

A language the scope settles is derived, and one the convention names is
pinned, in the words the rest of klin uses.

### Each convention is its own gate

A convention is judged as `conventions/<name>`, through the one ratchet and
the one accepted list. Two conventions on one line are two findings with two
remedies and two debts. The generic site identity does not change.

A `text` or `code` site is the file and the text of the line the match
starts on, with `count` ratcheted. A `files` site is the path. A site that
moves to another file is new, and no body hash pairs a site across files.

A file git renamed keeps its sites, as spec 4.4 and ADR 0009 have it, but the
scope reads the path each tree holds. A rename out of `except` is exactly the
move a convention forbids, so it brings its sites in as new. A `files` site is
the path each tree holds, so a rename into a glob is new.

An accepted entry whose gate names a convention the section no longer defines
is a configuration error, so debt cannot outlive the policy it was accepted
against in silence. A section that is absent or `false` runs no gate, and its
entries wait for it, as an excluded gate's entries do.

An `in` path the working tree holds nothing at leaves its convention
measuring nothing there. Outside the hook that is exit 2. In the hook it is a
NOTE, because the agent cannot edit the configuration. An `except` path that
names nothing takes nothing out, and is a NOTE.

### One walk, one read, one parse

A run walks each tree once, reads each file once for every `text` rule, and
parses each file once for every `code` rule. The walk is the shared one:
it skips the default skip set, hidden directories and what git ignores. The
configuration file is not walked: it states every literal a convention
forbids.

The adapter is `syntax::pattern`. It takes the grammar from the registry of
ADR 0035 and holds one thing per language, the character a hole is written
with. It hands `ast-grep-core` the tree `syntax` already parsed, and no type
of `ast-grep-core` leaves it.

## Consequences

`klin conventions --report` prints what each convention applies to, the
language and whether it was derived or pinned, what it matches now, each
site's outcome against the base, and what the convention cannot see. It
writes nothing.

`match $COMMAND { $$$ARMS, _ => Ok(0) }` and `RefCell<Records>` are not Rust
the grammar reads as one piece of code, so the rules #162 means to pin for
them are `text` rules.

ADR 0006's choice of `ast-grep-core` over `ast-grep-language` stands. Its
rule shape, `pattern`, `structural` and `example`, is superseded here.
