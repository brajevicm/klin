# One Git process seam and one atomic replacement primitive

Issue #161 completes the process and replacement-write consolidation left by
the earlier architecture decisions. This is a behavior-preserving refactor:
it adds no gate, changes no Git window or state format, and changes no report
text or failure policy.

## Decision

`git::Repo<'a> { root: &'a Path }` is the one concrete Git process adapter.
It owns `Command::new("git")`, `-C root`, captured output, lossy UTF-8 text,
blob bytes, `core.quotePath=false`, and environment overrides. Its interface is
small and semantic: text, single and batched blob reads, tree and ignored-path
listing, rev-parse paths, and worktree listing. There is no Git trait, mock, or
full Git object model.

Callers keep the policy that gives an invocation meaning. `changed` parses the
name-status diff and combines it with untracked paths; `hunks` parses changed
lines; `survey`, `files` and the file-level gates select and interpret paths;
`turn` owns the private index, stamp ref and authorship; `state` chooses its
rev-parse questions; `base` owns worktree materialization; and `stats` decides
what a worktree list means. A run still computes and reuses one `Project`
change set rather than issuing an equivalent diff from another caller.

`write::atomic_write` is the one replacement-content primitive. It writes a
temporary neighbor, optionally copies a source mode, renames over the target,
and removes the temporary best-effort on failure. `init` follows symlinks and
preserves the existing mode; the turn stamp and build-block record do not
preserve a mode. Journal append and the base's semantic move of an existing
path remain separate operations.

## Consequences

- Production Git launches are confined to `git::Repo`; callers no longer
  duplicate `-C`, output conversion, private-index setup, or batch blob
  plumbing.
- The batched base blob path is shared by all callers, while the run's change
  set and the survey's repository-wide path facts remain cached at their
  existing seams.
- Git probes keep their old `None` failure behavior. State writes remain
  non-blocking, `init` still returns its own error, and journal append remains
  best-effort.
- The helper preserves the existing `.writing` neighbor convention, so the
  old stamp and build-block failure paths continue to exercise the same
  atomic replacement boundary.

## Measurement

The #157 performance fixture ran before at commit `0ef58a3` and after this
change on 2026-09-14, macos/aarch64, release builds, five samples per row.
The rows use the existing 2k and 10k generated trees; hook timings exclude the
project build command. The guard row is 1,000 separate events.

| Row | Before (ms) | After (ms) | Change |
|---|---:|---:|---:|
| 2k warm hook | 1,644 | 1,764 | +7.3% |
| 2k cold survey | 1,960 | 2,085 | +6.4% |
| 2k strict | 1,304 | 1,334 | +2.3% |
| 10k warm hook | 6,811 | 7,092 | +4.1% |
| 10k cold survey | 16,890 | 14,527 | -14.0% |
| 10k strict | 6,881 | 6,251 | -9.2% |
| guard, 1,000 events | 10,248 | 9,630 | -6.0% |

The small warm-hook increase is measurable but not a timing cliff: it stays
below 8% on the 2k and 10k fixtures, while the larger cold and strict rows
improve. No timing-specific optimization is justified by these medians.

## Final self-enforcement

`conventions/single-git-boundary` scopes production source to `src` and
excepts `src/git.rs` and `src/changed.rs`, whose direct Git use is test-only;
integration tests are outside that scope. A new `Command::new("git")`
elsewhere gets the shared-boundary remedy.
