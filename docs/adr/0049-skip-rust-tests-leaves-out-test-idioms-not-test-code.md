# `skip_rust_tests` leaves out test idioms, not test code

> Amends spec 8.2. The key keeps its name and its default. What changes is
> what it leaves out, and where.

The blinded labeling of the two natural benchmark rounds (#262) put all five
of its `undesired` labels on one class: `escapes` interrupting an agent over
an `unwrap()` inside a Rust integration test under `tests/`. A person judged
that an `unwrap()` in a test is how a test asserts, so the interruption was
wrong. The same call inside an inline `#[cfg(test)]` module was already
silent, because `skip_rust_tests`, default `true`, dropped the whole line
range of such a module before any pattern matched. That range also hid
`#[ignore]`, `#[allow(...)]` and `unsafe { }`, so a test disabled inside a
module was never a finding, while the same attribute in a file under `tests/`
was.

Two readings closed the asymmetry. One drops every escape pattern from every
Rust test context, which is the smallest change and matches what the inline
rule did. The other leaves out only what a test writes on purpose and keeps
what a test uses to look green. The benchmark evidence supports the narrow
fact and no more: `unwrap()` in a test should not interrupt. It says nothing
in favour of hiding a disabled test, and klin's thesis names a skipped test as
an agent shortcut.

## The decision

**`skip_rust_tests` leaves out `unwrap` and `expect` inside Rust test code,
and nothing else.** Rust test code is an inline `#[cfg(test)]` module, which
the syntax convention classifier already finds, and a `.rs` file under a test
root the survey finds in the tree being read (spec 5.4). Each tree is
classified over its own files, not over the union of 4.3, because that union
keeps a root that was test-only at the derivation commit after production code
joined it, and would hide the production sites. Both contexts follow one rule. `#[ignore]`,
`#[allow(...)]` and `unsafe { }` inside either are findings, as they are
anywhere. With `skip_rust_tests: false`, `unwrap` and `expect` are judged in
test code too. Nothing changes for another language, for production Rust, or
for `stubs`, which still refuses the key because a stub in a test is a stub.

The rows a test may write are named on the check, beside its table, as the
`test_idioms` of the kind. The engine reads that list and the two contexts and
knows no pattern by name. No new configuration key distinguishes the two
readings: the evidence decided one, and a knob would let a tree measure a
different set in silence.

## Consequences

An inline test module that carried `#[allow(...)]` or `#[ignore]` under the
old rule now fails `escapes` where the base does not hold the site. That is
the reversal this record exists for. The coverage line counts the idioms left
out as `in Rust tests skipped`, without the word inline.
