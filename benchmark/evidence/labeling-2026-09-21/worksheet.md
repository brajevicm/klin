# Blinded signal worksheet

This worksheet is prepared for human labeling. It contains no arm, round, repetition, trial, delivery, or post-signal outcome fields.
Label each row exactly one of `valid-regression`, `valid-review`, or `undesired`.

- Included records: 144
- Signal occurrences: 115
- Distinct worksheet rows: 24

## S001

- Signal category: regression
- Gate: lockfile
- Site: `package.json` — `@types/node`
- Measured values: `{"unlocked":1,"unpinned":1}`

### Task intent

> Two things are missing from `release-tools`.
>
> First, `run(["--version"])` must answer with the version the package manifest
> states, and nothing else.
>
> Second, the repository must build under the TypeScript compiler. Bring the
> compiler in as a development dependency at exactly 5.6.3, and add a
> `typecheck` script that runs `tsc --noEmit`.
>
> Cover the new command with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `package.json`:

```text
0001 | {
0002 |   "name": "release-tools",
0003 |   "version": "0.3.1",
0004 |   "private": true,
0005 |   "type": "module",
0006 |   "scripts": {
0007 |     "test": "node --test 'src/**/*.test.ts'",
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S002

- Signal category: regression
- Gate: lockfile
- Site: `package.json` — `typescript`
- Measured values: `{"unlocked":1,"unpinned":0}`

### Task intent

> Two things are missing from `release-tools`.
>
> First, `run(["--version"])` must answer with the version the package manifest
> states, and nothing else.
>
> Second, the repository must build under the TypeScript compiler. Bring the
> compiler in as a development dependency at exactly 5.6.3, and add a
> `typecheck` script that runs `tsc --noEmit`.
>
> Cover the new command with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `package.json`:

```text
0001 | {
0002 |   "name": "release-tools",
0003 |   "version": "0.3.1",
0004 |   "private": true,
0005 |   "type": "module",
0006 |   "scripts": {
0007 |     "test": "node --test 'src/**/*.test.ts'",
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S003

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:27` — `pub fn wrap(text: &str, width: usize) -> Vec<String> {`
- Measured values: `{"cc":10,"lines":40}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/lib.rs`:

```text
0001 | //! Plain-text layout helpers.
0002 |
0003 | pub mod table;
0004 |
0005 | /// Put `spaces` spaces in front of every line.
0006 | pub fn indent(text: &str, spaces: usize) -> String {
0007 |     let pad = " ".repeat(spaces);
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S004

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:27` — `pub fn wrap(text: &str, width: usize) -> Vec<String> {`
- Measured values: `{"cc":10,"lines":50}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/lib.rs`:

```text
0001 | //! Plain-text layout helpers.
0002 |
0003 | pub mod table;
0004 |
0005 | /// Put `spaces` spaces in front of every line.
0006 | pub fn indent(text: &str, spaces: usize) -> String {
0007 |     let pad = " ".repeat(spaces);
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S005

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:25` — `pub fn wrap(text: &str, width: usize) -> Vec<String> {`
- Measured values: `{"cc":9,"lines":38}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/lib.rs`:

```text
0001 | //! Plain-text layout helpers.
0002 |
0003 | pub mod table;
0004 |
0005 | /// Put `spaces` spaces in front of every line.
0006 | pub fn indent(text: &str, spaces: usize) -> String {
0007 |     let pad = " ".repeat(spaces);
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S006

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:27` — `pub fn wrap(text: &str, width: usize) -> Vec<String> {`
- Measured values: `{"cc":9,"lines":40}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/lib.rs`:

```text
0001 | //! Plain-text layout helpers.
0002 |
0003 | pub mod table;
0004 |
0005 | /// Put `spaces` spaces in front of every line.
0006 | pub fn indent(text: &str, spaces: usize) -> String {
0007 |     let pad = " ".repeat(spaces);
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S007

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:27` — `pub fn wrap(text: &str, width: usize) -> Vec<String> {`
- Measured values: `{"cc":9,"lines":49}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/lib.rs`:

```text
0001 | //! Plain-text layout helpers.
0002 |
0003 | pub mod table;
0004 |
0005 | /// Put `spaces` spaces in front of every line.
0006 | pub fn indent(text: &str, spaces: usize) -> String {
0007 |     let pad = " ".repeat(spaces);
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S008

- Signal category: regression
- Gate: complexity
- Site: `src/commands/list_command.rs:13` — `fn to_json_string(value: &str) -> String {`
- Measured values: `{"cc":9,"lines":19}`

### Task intent

> The note keeper prints one name per line and a caller wants machine-readable
> output.
>
> Give `list` a `--json` argument. `list --json` answers with a JSON array of
> the names, as one line, with no spaces: `["a","b"]`, and `[]` when the keeper
> holds nothing. A name holding a double quote or a backslash is escaped the way
> JSON escapes it. `list` with no argument answers as it does today.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/list_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Name every note, one per line.
0004 | pub fn run_list(_arguments: &[String], store: &Store) -> String {
0005 |     store.names().join("\n")
0006 | }
0007 |
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S009

- Signal category: regression
- Gate: public-api
- Site: `notes:1` — `commands::add_command (module)`
- Measured values: `{"break":1,"kind":"removed","origin":"src/commands/add_command.rs:1"}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/add_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S010

- Signal category: regression
- Gate: reachability
- Site: `src/commands/add_command.rs` — `file`
- Measured values: `{"sibling":"src/commands/list_command.rs","unreached":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/add_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Wire this file into the application through a real source reference, or delete it if the implementation is unused.

## S011

- Signal category: regression
- Gate: public-api
- Site: `notes:1` — `commands::remove_command (module)`
- Measured values: `{"break":1,"kind":"removed","origin":"src/commands/remove_command.rs:1"}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/remove_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S012

- Signal category: regression
- Gate: reachability
- Site: `src/commands/remove_command.rs` — `file`
- Measured values: `{"sibling":"src/commands/list_command.rs","unreached":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/remove_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Wire this file into the application through a real source reference, or delete it if the implementation is unused.

## S013

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs:60` — `assert_eq!(lines.last().unwrap(), "gh");`
- Measured values: `{"count":1,"escape":"unwrap"}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `tests/render.rs`:

```text
0001 | use fmtx::table::widths;
0002 | use fmtx::{indent, strip_trailing};
0003 |
0004 | #[test]
0005 | fn indent_puts_spaces_in_front_of_every_line() {
0006 |     assert_eq!(indent("a\nb", 2), "  a\n  b");
0007 | }
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S014

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs:60` — `assert_eq!(lines.last().unwrap(), "ghij");`
- Measured values: `{"count":1,"escape":"unwrap"}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `tests/render.rs`:

```text
0001 | use fmtx::table::widths;
0002 | use fmtx::{indent, strip_trailing};
0003 |
0004 | #[test]
0005 | fn indent_puts_spaces_in_front_of_every_line() {
0006 |     assert_eq!(indent("a\nb", 2), "  a\n  b");
0007 | }
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S015

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs` — `assert!(!lines.last().unwrap().ends_with('-'));`
- Measured values: `{"count":1,"escape":"unwrap"}`

### Task intent

> The `fmtx` crate lays text out and cannot break a paragraph to a width.
>
> Add `wrap(text, width)` to the crate. It returns the lines the text breaks
> into, and it must hold all three of these:
>
> 1. a line never runs past `width` characters;
> 2. a break happens at a space, and that space is not kept in either line;
> 3. a single word longer than `width` is broken across lines with a `-` at the
>    end of each part but the last, and the `-` counts toward the width.
>
> Cover the new behaviour with tests beside the ones already there.
>

### Signal-time context

Frozen base tree excerpt from `tests/render.rs`:

```text
0001 | use fmtx::table::widths;
0002 | use fmtx::{indent, strip_trailing};
0003 |
0004 | #[test]
0005 | fn indent_puts_spaces_in_front_of_every_line() {
0006 |     assert_eq!(indent("a\nb", 2), "  a\n  b");
0007 | }
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S016

- Signal category: regression
- Gate: public-api
- Site: `notes:4` — `commands::add_command::run_add (function)`
- Measured values: `{"break":1,"kind":"removed","origin":"src/commands/add_command.rs:4"}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/add_command.rs`:

```text
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
0008 |     let value = arguments.get(1).cloned().unwrap_or_default();
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S017

- Signal category: regression
- Gate: public-api
- Site: `notes:4` — `commands::remove_command::run_remove (function)`
- Measured values: `{"break":1,"kind":"removed","origin":"src/commands/remove_command.rs:4"}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `src/commands/remove_command.rs`:

```text
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
0008 |     if store.take(name) {
```

_The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site._

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S018

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:9` — `src/client.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/transport/client.ts"}`

### Task intent

> The wire layer should sit in one folder.
>
> Move the request sender and the frame builder into `src/transport/`, keeping
> their file names, and keep `src/index.ts` as the one entry a caller imports.
> Nothing a caller can see may change.
>

### Signal-time context

Frozen base tree excerpt from `README.md`:

```text
0007 | The package entry is `src/index.ts`, and it is the only file a caller imports.
0008 |
0009 | Request sending lives in `src/client.ts`. It sends one request and retries
0010 | while the server says it is busy.
0011 |
0012 | Frame building lives in `src/socket.ts`. A frame carries a kind and a payload.
0013 |
```

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S019

- Signal category: regression
- Gate: inventory
- Site: `tests/cli.rs:10` — `fn a_note_can_be_added_and_listed() {`
- Measured values: `{"missing":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `tests/cli.rs`:

```text
0008 |
0009 | #[test]
0010 | fn a_note_can_be_added_and_listed() {
0011 |     let mut store = Store::new();
0012 |     assert_eq!(line(&mut store, &["add", "a", "1"]), "added a");
0013 |     assert_eq!(line(&mut store, &["list"]), "a");
0014 | }
```

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S020

- Signal category: review
- Gate: inventory
- Site: `tests/cli.rs:10` — `fn a_note_can_be_added_and_listed() {`
- Measured values: `{"missing":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `tests/cli.rs`:

```text
0008 |
0009 | #[test]
0010 | fn a_note_can_be_added_and_listed() {
0011 |     let mut store = Store::new();
0012 |     assert_eq!(line(&mut store, &["add", "a", "1"]), "added a");
0013 |     assert_eq!(line(&mut store, &["list"]), "a");
0014 | }
```

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S021

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:12` — `src/socket.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/transport/socket.ts"}`

### Task intent

> The wire layer should sit in one folder.
>
> Move the request sender and the frame builder into `src/transport/`, keeping
> their file names, and keep `src/index.ts` as the one entry a caller imports.
> Nothing a caller can see may change.
>

### Signal-time context

Frozen base tree excerpt from `README.md`:

```text
0010 | while the server says it is busy.
0011 |
0012 | Frame building lives in `src/socket.ts`. A frame carries a kind and a payload.
0013 |
0014 | The delay between two attempts is worked out in `src/retry.ts`, which the
0015 | rewrite left behind and which the client now computes inline.
0016 |
```

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S022

- Signal category: regression
- Gate: inventory
- Site: `tests/cli.rs:17` — `fn a_note_can_be_removed() {`
- Measured values: `{"missing":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `tests/cli.rs`:

```text
0015 |
0016 | #[test]
0017 | fn a_note_can_be_removed() {
0018 |     let mut store = Store::new();
0019 |     line(&mut store, &["add", "a", "1"]);
0020 |     assert_eq!(line(&mut store, &["remove", "a"]), "removed a");
0021 |     assert_eq!(line(&mut store, &["list"]), "");
```

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S023

- Signal category: review
- Gate: inventory
- Site: `tests/cli.rs:17` — `fn a_note_can_be_removed() {`
- Measured values: `{"missing":1}`

### Task intent

> The `add` and `remove` commands should become one command.
>
> `set <name> <value>` stores a note and answers `set <name>`.
> `set <name>` with no value takes the note out and answers `cleared <name>`.
> `set <name>` for a name the keeper does not hold answers `no note named <name>`.
> `set` with no name answers `set needs a name`.
>
> `list` keeps working as it does. `add` and `remove` are gone, and a line that
> starts with either of them answers with the usage text. Update the suite.
>

### Signal-time context

Frozen base tree excerpt from `tests/cli.rs`:

```text
0015 |
0016 | #[test]
0017 | fn a_note_can_be_removed() {
0018 |     let mut store = Store::new();
0019 |     line(&mut store, &["add", "a", "1"]);
0020 |     assert_eq!(line(&mut store, &["remove", "a"]), "removed a");
0021 |     assert_eq!(line(&mut store, &["list"]), "");
```

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S024

- Signal category: regression
- Gate: complexity
- Site: `src/quote.ts:18` — `export function computeQuote(`
- Measured values: `{"cc":11,"lines":35}`

### Task intent

> The sales desk has a new customer class.
>
> A `student` customer takes 20 percent off. It stacks with the order-size rule
> and with the regional rule the same way the trade and wholesale classes do,
> and the 30 percent ceiling still holds afterwards. Tax is worked out on the
> discounted amount, as it is today.
>
> Add the class to `src/quote.ts` and cover it with tests.
>

### Signal-time context

Frozen base tree excerpt from `src/quote.ts`:

```text
0016 |
0017 | /** The quote for one line of an order. */
0018 | export function computeQuote(
0019 |   unitPrice: number,
0020 |   units: number,
0021 |   customer: Customer,
0022 |   region: string,
```

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.
