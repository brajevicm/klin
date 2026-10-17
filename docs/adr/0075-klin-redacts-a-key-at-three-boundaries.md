# klin redacts a key at three boundaries

> Recorded for #653, 2026-10-17, from the owner's decision of the same day,
> after four review rounds on PR #701.

The `secrets` check (#582) keys its findings by a fingerprint, the vendor
prefix, `...` and a hash. Other checks still named the source line, so a key
on that line reached the text report, the check document, the journal and
an accepted entry a person copied. A key also reaches klin from file names,
SARIF reports, build output, `klin.json` and hook events, and klin echoes
each of those somewhere.

## The decision

klin redacts at three places, and not at each source:

1. **Identity.** `Evaluator::evaluate` replaces each token in a finding's
   file and text with its fingerprint before it matches, and reads the
   changed paths of a scoped run the same way. The identity hashes the
   redacted file and text, so no id holds a key, no accepted entry needs
   one, and an entry copied from a report matches the site it names.
2. **Output.** `sys::out` is the only code that writes to standard output or
   standard error. It redacts every text first.
3. **Records.** `hook::record` is the only code in `src/hook` that writes a
   file. It redacts a record for a person or a tool: the journal and
   `KLIN_HOOK_REPORT`. It writes machine state that klin reads back and
   compares as it is: the turn stamp, the handoff, the budget and the told
   marker. A redacted branch name would no longer match the branch on the
   next event. The stamp's one text, the error of an `unjudged` verdict, is
   redacted where `Verdict::unjudged` makes it.

Conventions in klin's own `klin.json` enforce the second and third places:
`single-output-boundary` and `single-output-boundary-lines` forbid `print!(`
and `println!(` in `src` outside `src/sys/out.rs`, and the three
`single-record-writer` rules forbid `fs::write(`, `OpenOptions::new(` and
`atomic_write(` in `src/hook` outside `src/hook/record.rs`. A new print or
write that skips redaction fails klin's own gate on the next Stop.

The token table and the site hash live in `sys`, below every layer that
prints or writes.

One invariant follows from the boundaries. Any value that klin compares,
recognizes or hashes against text it already printed or stored redacted is
redacted the same way first. Three places need it: the ratchet matches an
accepted entry a person copied from a report, the handoff hashes a Cursor
follow-up as `sys::out` prints it, and a Stop looks up its session in the
journal as `hook::record` stored it.

Redaction reads a looser pattern than the `secrets` gate counts with:

- No word boundary before a token. A boundary would miss a key that JSON
  escaping runs into, such as the `n` of `\nsk-proj-…`, and the output
  boundary sees serialized JSON. A token's own characters never need
  escaping.
- A vendor's prefix and 8 or more characters of its rest, not the usual
  length. A boundary sees text that an excerpt or a clip already cut, and a
  cut key is shorter than the strict pattern. At most 7 characters of a cut
  key can show.
- A token that looks like a placeholder, which the gate does not count.

The rules ban the print macros and three ways to write a file. They do not
see a write to `io::stdout()` or a panic's message, which Rust's own panic
hook prints. Neither path carries text from a finding today.

## Rejected

- **Redact at each source or each sink by hand.** The first version of
  PR #701 did this. Four review rounds each found a print or a write that
  the last round missed: a config error, an unmatched accepted entry, a
  check document field, a build's output, a plan error, a `--host`
  argument. About twelve calls covered the paths found so far, and nothing
  stopped a new path from skipping them.
- **A `Redacted` newtype that every print and write must take.** The
  compiler would enforce it, but every caller would need to change its
  types. The conventions give the same guarantee with klin's own gate, and
  the change stays small.
- **Redact each excerpt and clip before it cuts.** Every place that cuts
  text would need its own call, which is the same problem as the first
  option. The loose pattern covers all of them at the boundary.
- **Redact the caches under `.git/klin`.** They keep file paths as lookup
  keys, so a redacted path would no longer find its entry. klin never prints
  or exports them. The spec states this as a known limit.
