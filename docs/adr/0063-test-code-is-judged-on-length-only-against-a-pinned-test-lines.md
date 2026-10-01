# Test code is judged on length only against a pinned `test_lines`

> Amends ADR 0016 and spec 5.3, 5.4, 5.7, 7.1 and 8.2.1. A missing
> `complexity.test_lines` is not judged, not derived. ADR 0062 Q2 stands for
> every function a length ceiling judges.

ADR 0016 says a value the configuration does not pin is derived. `lines`
held the length of every function, tests included. The replay of #343
(`docs/false-alarms-2026-09-29.md`) labeled long test bodies as not
appropriate findings: R040, a 245-line test body, R042, a fixture helper that
grew from 58 to 62 lines, and the roughly 100 long test bodies of R054. An
agent drafted those labels and agents reviewed them.

## The decision

**Test code is judged on `cc` as before, and on length only against
`complexity.test_lines`.** Test code is a test file of spec 5.4, one under a
test root or one a test directory segment or a test affix marks, and a Rust
item marked `#[cfg(test)]`, such as an inline test module, helpers and
fixtures included. Such an item is compiled only for tests. It is the
code `skip_test_idioms` reads as test code (ADR 0060), with a test file of
5.4 counted in every language.

- `test_lines` is a whole number or dated steps (spec 5.5). A person pins it.
  klin never derives it, so this amends ADR 0016: a missing `test_lines`
  means test code is not judged on length.
- `lines` means production code only.
- With no `test_lines`, a function in test code ratchets `cc` alone, so a
  test over its `cc` ceiling does not worsen when only its length grows. ADR
  0062 Q2 still holds wherever a length ceiling judges the function.
- A value a finding carries and its matched entry does not is a rise. A
  function that leaves test code starts to carry `lines`, and it is judged on
  its length from that point, so leaving test code cannot keep a length the
  production ceiling refuses.
- Each tree is classified over its own files, and a renamed file is
  classified at the base under its base path.
- A finding in test code carries its length as `test_lines`, and one in
  production code as `lines`. A site over its `cc` ceiling that moves between
  the two carries a value its base entry does not, so it is worsened,
  whichever length ceiling is stricter and even when its length is under
  both. The rule fails closed: a move cannot keep a length the new class
  refuses.
- An accepted `complexity` entry may leave out `lines` or `test_lines`. It
  then holds only a finding that does not carry the value it left out.
- The `OK:` line names added and renamed files from the change set, so a
  file a wider scope brings in is not named.
- Test code stays in the derived sample of `cc` and `lines`, so no derived
  ceiling moves.
- With no `test_lines`, the `OK:` line of `complexity` says how many test
  functions were not judged on length, and names each file holding one that
  the change added or renamed.
- `init --pin` writes no `test_lines`, because it pins only derived values.

## Known costs

- Agents tend to write verbose code. SlopCodeBench found verbosity rising in
  75.5% of trajectories. With `{}`, nothing checks the length of the tests an
  agent writes. The coverage line names the new test files so a person sees
  where that length went.
- The affix table marks some production files as tests, such as a `test_`
  prefix, a `Test` suffix like `SpeedTest.swift`, or a `spec/` segment.
  Moving code into such a file avoids the length check. It does not avoid
  `cc`.

## Consequences

A repository that wants its tests judged on length pins `test_lines`. klin's
own `klin.json` needs `"test_lines": 60` to judge its tests as it did before.
An agent may not write that line, so the person who merges this adds it.
