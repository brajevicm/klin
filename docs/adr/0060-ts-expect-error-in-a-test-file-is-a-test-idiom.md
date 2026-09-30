# `@ts-expect-error` in a test file is a test idiom

> Amends ADR 0049 and spec 5.3 and 8.2. The rule that a test idiom is left out
> of test code, and every other row is judged there, stands. What changes is
> the key's name, the set of idioms, and where each language says its test
> code is.

ADR 0049 left `unwrap` and `expect` out of Rust test code, because a test
asserts that way on purpose. TypeScript has the same kind of line. A type test
writes `// @ts-expect-error` above a line that must not compile, and the
comment fails the build when the line starts to compile. The comment is the
assertion.

The replay of #343 (`docs/false-alarms-2026-09-29.md`) labeled 131 findings of
R052 in type tests and test files as not appropriate. An agent drafted those
labels and agents reviewed them. The `ts-ignore` row matched
`@ts-expect-error` together with `@ts-ignore` and `@ts-nocheck`, so the row
could not tell the assertion apart from the escapes.

## The decision

**`@ts-expect-error` in a TypeScript or JavaScript test file is a test idiom.**
It leaves the `ts-ignore` row and becomes a row of its own,
`ts-expect-error`. A test file is what spec 5.4 calls one: a file under a test
root, or a file a test directory segment or a test affix marks. The comment
needs no description, the same way a reason on `#[ignore]` silences nothing.

- `@ts-ignore`, `@ts-nocheck` and the non-null `!` stay escapes in test
  files. They silence the checker and assert nothing.
- `@ts-expect-error` stays an escape in production code.

**The key `skip_rust_tests` becomes `skip_test_idioms`,** default `true`. It
governs every language's test idioms: the Rust `unwrap` and `expect`, and this
one. A section that still names `skip_rust_tests` gets the migration error of
spec 5.2, which names `skip_test_idioms`. The coverage line says
`in tests skipped`, without the word Rust.

**Each language's table names its test idioms and its test code.** ADR 0049
put the idiom list on the kind. This record moves it onto each language, with
the rule that says where that language's test code is: Rust's inline
`#[cfg(test)]` modules and test roots, or the test files of 5.4. The engine
reads the table and knows no idiom or language by name, so a new language's
idiom is a new table entry and no change to the engine. Rust keeps its own
rule. A `.rs` file that only a test affix or a directory segment marks, such
as a file under `src/spec/`, is not Rust test code, as ADR 0049 decided.

## Rejected

- **Keep `@ts-expect-error` in the `ts-ignore` row.** The row cannot then
  tell the assertion apart from the escapes, and the evidence is that the
  assertion is noise in a test.
- **Leave out every TypeScript escape in a test file.** ADR 0049 rejected the
  same reading for Rust. The replay labeled non-null assertions in tests (R067,
  R074) as not appropriate too, and #389 decided that `!` stays an escape in
  test files all the same, with `@ts-ignore` and `@ts-nocheck`. A person who
  judges one such site safe accepts it in the `accepted` list.
- **Require a description after `@ts-expect-error`.** No description is
  required, the same way a reason on `#[ignore]` silences nothing.
- **Keep the name `skip_rust_tests`.** The key would govern a TypeScript
  idiom under a Rust name. #389 chose the rename before 0.4.

## Consequences

A `klin.json` that names `skip_rust_tests` stops working until a person
renames the key. An accepted `escapes` entry that names the `ts-ignore` row
for a `@ts-expect-error` line no longer matches that line, because the line
now carries the `ts-expect-error` row.
