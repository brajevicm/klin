# Blinded signal worksheet

This worksheet is prepared for human labeling. It contains no arm, round, repetition, trial, delivery, or post-signal outcome fields.
Label each row exactly one of `valid-regression`, `valid-review`, or `undesired`.

- Included records: 144
- Signal occurrences: 115
- Distinct worksheet rows: 38

## S001

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

Signal-time tree excerpt from `src/commands/add_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> notes:1  removed, declared at src/commands/add_command.rs:1  commands::add_command (module)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S002

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

Signal-time tree excerpt from `src/commands/add_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> src/commands/add_command.rs:0  unreached, a reached sibling is src/commands/list_command.rs, was reached  file  — matched the base site at src/commands/add_command.rs:0

### Proposed remedy or review question

> Wire this file into the application through a real source reference, or delete it if the implementation is unused.

## S003

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

Signal-time tree excerpt from `src/commands/remove_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> notes:1  removed, declared at src/commands/remove_command.rs:1  commands::remove_command (module)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S004

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

Signal-time tree excerpt from `src/commands/remove_command.rs`:

```text
0001 | use crate::store::Store;
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> src/commands/remove_command.rs:0  unreached, a reached sibling is src/commands/list_command.rs, was reached  file  — matched the base site at src/commands/remove_command.rs:0

### Proposed remedy or review question

> Wire this file into the application through a real source reference, or delete it if the implementation is unused.

## S005

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

Signal-time tree excerpt from `src/commands/add_command.rs`:

```text
0001 |
```

Signal-time measurement:

> notes:1  removed, declared at src/commands/add_command.rs:1  commands::add_command (module)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S006

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

Signal-time tree excerpt from `src/commands/remove_command.rs`:

```text
0001 |
```

Signal-time measurement:

> notes:1  removed, declared at src/commands/remove_command.rs:1  commands::remove_command (module)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S007

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

Signal-time tree excerpt from `src/commands/add_command.rs`:

```text
0002 |
0003 | /// Store one note, and say what happened.
0004 | pub fn run_add(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "add needs a name".to_string();
0007 |     };
0008 |     let value = arguments.get(1).cloned().unwrap_or_default();
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> notes:4  removed, declared at src/commands/add_command.rs:4  commands::add_command::run_add (function)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S008

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

Signal-time tree excerpt from `src/commands/remove_command.rs`:

```text
0002 |
0003 | /// Take one note out, and say what happened.
0004 | pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
0005 |     let Some(name) = arguments.first() else {
0006 |         return "remove needs a name".to_string();
0007 |     };
0008 |     if store.take(name) {
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> notes:4  removed, declared at src/commands/remove_command.rs:4  commands::remove_command::run_remove (function)  — nothing matched

### Proposed remedy or review question

> Restore the removed surface or item, or keep the declared contract it had at the base. A break a person means is an accepted entry, written in a reviewed commit.

## S009

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

Signal-time tree excerpt from `README.md`:

```text
0007 | The package entry is `src/index.ts`, and it is the only file a caller imports.
0008 |
0009 | Request sending lives in `src/client.ts`. It sends one request and retries
0010 | while the server says it is busy.
0011 |
0012 | Frame building lives in `src/socket.ts`. A frame carries a kind and a payload.
0013 |
```

Signal-time measurement:

> README.md:9  not under the roots — likely src/transport/client.ts  src/client.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S010

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

Signal-time tree excerpt from `tests/cli.rs`:

```text
0008 |
0009 | #[test]
0010 | fn a_note_can_be_added_and_listed() {
0011 |     let mut store = Store::new();
0012 |     assert_eq!(line(&mut store, &["add", "a", "1"]), "added a");
0013 |     assert_eq!(line(&mut store, &["list"]), "a");
0014 | }
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> tests/cli.rs:10  missing 1, was missing 0  fn a_note_can_be_added_and_listed() {  — matched the base site at tests/cli.rs:10

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S011

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

Signal-time tree excerpt from `tests/cli.rs`:

```text
0008 |
0009 | #[test]
0010 | fn a_note_can_be_added_and_listed() {
0011 |     let mut store = Store::new();
0012 |     assert_eq!(line(&mut store, &["add", "a", "1"]), "added a");
0013 |     assert_eq!(line(&mut store, &["list"]), "a");
0014 | }
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> tests/cli.rs:10  missing 1, was missing 0  fn a_note_can_be_added_and_listed() {  — matched the base site at tests/cli.rs:10

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S012

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

Signal-time tree excerpt from `package.json`:

```text
0010 |   },
0011 |   "devDependencies": {
0012 |     "@types/node": "^20.14.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 1  @types/node  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S013

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

Signal-time tree excerpt from `package.json`:

```text
0010 |   },
0011 |   "devDependencies": {
0012 |     "@types/node": "^22.0.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 1  @types/node  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S014

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

Signal-time tree excerpt from `package.json`:

```text
0010 |   },
0011 |   "devDependencies": {
0012 |     "@types/node": "^22.7.5",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 1  @types/node  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S015

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

Signal-time tree excerpt from `package.json`:

```text
0010 |   },
0011 |   "devDependencies": {
0012 |     "@types/node": "^22.9.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 1  @types/node  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S016

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

Signal-time tree excerpt from `README.md`:

```text
0010 | while the server says it is busy.
0011 |
0012 | Frame building lives in `src/socket.ts`. A frame carries a kind and a payload.
0013 |
0014 | The delay between two attempts is worked out in `src/retry.ts`, which the
0015 | rewrite left behind and which the client now computes inline.
0016 |
```

Signal-time measurement:

> README.md:12  not under the roots — likely src/transport/socket.ts  src/socket.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S017

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

Signal-time tree excerpt from `package.json`:

```text
0011 |   "devDependencies": {
0012 |     "eslint": "^9.0.0",
0013 |     "@types/node": "^22.7.4",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 1  @types/node  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S018

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

Signal-time tree excerpt from `package.json`:

```text
0011 |   "devDependencies": {
0012 |     "eslint": "^9.0.0",
0013 |     "typescript": "5.6.3"
0014 |   }
0015 | }
0016 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S019

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

Signal-time tree excerpt from `src/commands/list_command.rs`:

```text
0011 | }
0012 |
0013 | fn to_json_string(value: &str) -> String {
0014 |     let mut result = String::with_capacity(value.len() + 2);
0015 |     result.push('"');
0016 |     for character in value.chars() {
0017 |         match character {
```

Signal-time measurement:

> src/commands/list_command.rs:13  cc 9, 19 lines  fn to_json_string(value: &str) -> String {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S020

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

Signal-time tree excerpt from `package.json`:

```text
0012 |     "@types/node": "^20.14.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S021

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

Signal-time tree excerpt from `package.json`:

```text
0012 |     "@types/node": "^22.0.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S022

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

Signal-time tree excerpt from `package.json`:

```text
0012 |     "@types/node": "^22.7.5",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S023

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

Signal-time tree excerpt from `package.json`:

```text
0012 |     "@types/node": "^22.9.0",
0013 |     "eslint": "^9.0.0",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S024

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

Signal-time tree excerpt from `package.json`:

```text
0012 |     "eslint": "^9.0.0",
0013 |     "@types/node": "^22.7.4",
0014 |     "typescript": "5.6.3"
0015 |   }
0016 | }
0017 |
```

Signal-time measurement:

> package.json:0  unlocked 1, unpinned 0  typescript  — nothing matched

### Proposed remedy or review question

> Run the project's own install so the lockfile records the dependency, and give the specifier the base's exact version back. A dependency the lockfile does not know is one no install has ever resolved, and a pin the base held is a version a person chose.

## S025

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

Signal-time tree excerpt from `tests/cli.rs`:

```text
0015 |
0016 | #[test]
0017 | fn a_note_can_be_removed() {
0018 |     let mut store = Store::new();
0019 |     line(&mut store, &["add", "a", "1"]);
0020 |     assert_eq!(line(&mut store, &["remove", "a"]), "removed a");
0021 |     assert_eq!(line(&mut store, &["list"]), "");
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> tests/cli.rs:17  missing 1, was missing 0  fn a_note_can_be_removed() {  — matched the base site at tests/cli.rs:17

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S026

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

Signal-time tree excerpt from `tests/cli.rs`:

```text
0015 |
0016 | #[test]
0017 | fn a_note_can_be_removed() {
0018 |     let mut store = Store::new();
0019 |     line(&mut store, &["add", "a", "1"]);
0020 |     assert_eq!(line(&mut store, &["remove", "a"]), "removed a");
0021 |     assert_eq!(line(&mut store, &["list"]), "");
```

_The signal-time tree does not contain the reported text; this is the frozen base context at the recorded site._

Signal-time measurement:

> tests/cli.rs:17  missing 1, was missing 0  fn a_note_can_be_removed() {  — matched the base site at tests/cli.rs:17

### Proposed remedy or review question

> If a test failed because the code is wrong, restore the test and fix the code. If the removal is intended, say why in your reply and stop again.

## S027

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

Signal-time tree excerpt from `src/quote.ts`:

```text
0016 |
0017 | /** The quote for one line of an order. */
0018 | export function computeQuote(
0019 |   unitPrice: number,
0020 |   units: number,
0021 |   customer: Customer,
0022 |   region: string,
```

Signal-time measurement:

> src/quote.ts:18  cc 11, 35 lines, was cc 10, 33 lines  export function computeQuote(  — matched the base site at src/quote.ts:18, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S028

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0023 | /// (which are dropped) and, for a single word longer than `width`,
0024 | /// hyphenating it across as many lines as needed.
0025 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0026 |     let mut lines = Vec::new();
0027 |     let mut current = String::new();
0028 |
0029 |     for mut word in text.split_whitespace() {
```

Signal-time measurement:

> src/lib.rs:25  cc 9, 38 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S029

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0025 | /// itself broken across lines, with a `-` (counted toward `width`) ending
0026 | /// every part but the last.
0027 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0028 |     let mut lines = Vec::new();
0029 |     let mut current = String::new();
0030 |     let mut current_len = 0;
0031 |
```

Signal-time measurement:

> src/lib.rs:27  cc 10, 40 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S030

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0025 | /// kept on either side. A word longer than `width` is split across lines
0026 | /// with a trailing `-`, which counts toward that line's width.
0027 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0028 |     let mut lines: Vec<String> = Vec::new();
0029 |     let mut current: Vec<char> = Vec::new();
0030 |
0031 |     for word in text.split_whitespace() {
```

Signal-time measurement:

> src/lib.rs:27  cc 10, 40 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S031

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0025 | /// than `width` is split across lines, with a `-` (counted toward the
0026 | /// width) ending every part but the last.
0027 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0028 |     let mut lines = Vec::new();
0029 |     let mut current = String::new();
0030 |     let mut current_len = 0usize;
0031 |
```

Signal-time measurement:

> src/lib.rs:27  cc 9, 49 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S032

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0025 | /// than `width` on its own is split across lines with a trailing `-`, which
0026 | /// counts toward that line's width.
0027 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0028 |     let mut lines = Vec::new();
0029 |     let mut current = String::new();
0030 |
0031 |     for mut word in text.split(' ').filter(|word| !word.is_empty()) {
```

Signal-time measurement:

> src/lib.rs:27  cc 9, 40 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S033

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

Signal-time tree excerpt from `src/lib.rs`:

```text
0025 | /// than kept at the end or start of a line. A word longer than `width` is
0026 | /// broken across lines with a trailing `-`, which counts toward the width.
0027 | pub fn wrap(text: &str, width: usize) -> Vec<String> {
0028 |     let mut lines = Vec::new();
0029 |     let mut current = String::new();
0030 |     let mut current_len = 0;
0031 |
```

Signal-time measurement:

> src/lib.rs:27  cc 10, 50 lines  pub fn wrap(text: &str, width: usize) -> Vec<String> {  — nothing matched, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S034

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs:50` — `assert!(!lines.last().unwrap().ends_with('-'));`
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

Signal-time tree excerpt from `tests/render.rs`:

```text
0048 |     let lines = wrap("abcdefgh", 3);
0049 |     assert_eq!(lines, vec!["ab-", "cd-", "ef-", "gh"]);
0050 |     assert!(!lines.last().unwrap().ends_with('-'));
0051 | }
0052 |
0053 | #[test]
0054 | fn wrap_of_empty_text_is_no_lines() {
```

Signal-time measurement:

> tests/render.rs:50  unwrap  assert!(!lines.last().unwrap().ends_with('-'));  — nothing matched

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S035

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs:56` — `assert!(!lines.last().unwrap().ends_with('-'));`
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

Signal-time tree excerpt from `tests/render.rs`:

```text
0054 |     let lines = wrap("abcdefghij", 4);
0055 |     assert!(lines[..lines.len() - 1].iter().all(|l| l.ends_with('-')));
0056 |     assert!(!lines.last().unwrap().ends_with('-'));
0057 | }
0058 |
0059 | #[test]
0060 | fn wrap_mixes_normal_words_and_a_long_word() {
```

Signal-time measurement:

> tests/render.rs:56  unwrap  assert!(!lines.last().unwrap().ends_with('-'));  — nothing matched

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S036

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

Signal-time tree excerpt from `tests/render.rs`:

```text
0058 | fn wrap_does_not_dash_the_last_part_of_a_split_word() {
0059 |     let lines = wrap("abcdefghij", 4);
0060 |     assert_eq!(lines.last().unwrap(), "ghij");
0061 | }
0062 |
0063 | #[test]
0064 | fn wrap_resumes_normal_wrapping_after_a_split_word() {
```

Signal-time measurement:

> tests/render.rs:60  unwrap  assert_eq!(lines.last().unwrap(), "ghij");  — nothing matched

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S037

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

Signal-time tree excerpt from `tests/render.rs`:

```text
0058 | fn wrap_only_hyphenates_the_parts_before_the_last() {
0059 |     let lines = wrap("abcdefgh", 4);
0060 |     assert_eq!(lines.last().unwrap(), "gh");
0061 |     assert!(!lines.last().unwrap().ends_with('-'));
0062 | }
0063 |
0064 | #[test]
```

Signal-time measurement:

> tests/render.rs:60  unwrap  assert_eq!(lines.last().unwrap(), "gh");  — nothing matched

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## S038

- Signal category: regression
- Gate: escapes
- Site: `tests/render.rs:61` — `assert!(!lines.last().unwrap().ends_with('-'));`
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

Signal-time tree excerpt from `tests/render.rs`:

```text
0059 |     let lines = wrap("abcdefgh", 4);
0060 |     assert_eq!(lines.last().unwrap(), "gh");
0061 |     assert!(!lines.last().unwrap().ends_with('-'));
0062 | }
0063 |
0064 | #[test]
0065 | fn wrap_of_empty_text_is_no_lines() {
```

Signal-time measurement:

> tests/render.rs:61  unwrap  assert!(!lines.last().unwrap().ends_with('-'));  — nothing matched

### Proposed remedy or review question

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.
