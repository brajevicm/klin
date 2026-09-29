# klin.json is policy over derived facts

> ADR 0048 amends this: the derived build line names the manifest each
> command came from, and the hook prints it when the build fails too.
>
> #382 amends the `doc_size` bullet: only `AGENTS.md` and `CLAUDE.md` at the
> tree root keep a derived ceiling, and the map must pin any other document.
> None of the 16 doc-size rows of the #343 replay was appropriate, and all of
> them were changelogs, READMEs or other reader documents
> (`docs/false-alarms-2026-09-29.md`).
>
> Amends ADR 0005, 0012 and 0016 where they have `init` write derived
> sections, and completes ADR 0038 and 0039.

ADR 0038 split the policy a person writes from the facts a tree holds, and
kept a compatibility layer: `survey::Derived` still manufactured one JSON
section per derivable check, merged a person's pins over it key by key, and
printed the provenance of every section before any gate ran. ADR 0039 moved
the five source checks off that layer. Four checks and the build still read
it: `doc_size` and `doc_citations` read generated document entries,
`inventory` read generated test-root entries, `lockfile` read a generated
`manifests` list, and the hook read a generated `build` command. `init` wrote
those sections into `klin.json`, with `project` and the binary's `version`,
so a configuration was a serialized picture of the repository on one day.
Issue #180 is the contract step: the layer goes.

## The decision

**`klin.json` states only what a person decided.** `{}` is a complete
configuration. A section is absent, `false`, or a person's policy:

- `doc_size` is a map of document path to ceiling, a whole number or a dated
  schedule. A pin names its document by path; every Markdown file at the tree
  root the map does not name keeps its derived ceiling, so pinning one
  document takes no other out of scrutiny.
- `doc_citations` reads no policy. Every Markdown file at the tree root is
  read against the whole tree with the built-in extension list. No
  resolution DSL replaces the retired `file`, `roots` and `extensions`; a
  repository that proves it needs another root earns the smallest explicit
  exception then.
- `inventory` reads `in` and `except`. A test is a file under a test root the
  survey finds, or a source file a test directory segment or a test affix
  marks wherever it sits, so a Go test beside its package needs no entry.
  Both trees are read under the scope the base commit records, and under
  today's when the base records none, so a narrowing lets a deletion through
  only once it is committed.
- `lockfile` reads `in` and `except`. Its manifests are the ones the survey
  finds that klin has a reader for. Every manifest and lockfile is read once
  per tree, the base's through one `git cat-file` process, and a lockfile
  several manifests share is parsed once.
- `build` is an override: a command, a list of `{run, root}` entries, or
  `false`. Absent, the hook derives one command per standard manifest when it
  builds, and prints it. No other run derives a build.

`project` and `version` are no longer keys. The repository's identity is a
fact, and a binary version froze no semantics. If klin ever needs a schema
epoch, that is its own decision. Every retired field, and every field a
section does not read, is refused before any gate runs, with the replacement
named, and a misspelling names the field a person most likely meant.
`accepted`, `radius` and `journal` keep their meaning; `radius`, `journal`
and `build` entries are now refused an unknown field too.

**Each check resolves its own policy and says where each value came from.**
`survey` holds facts only: the derivation commit's survey of roots,
documents, test roots and manifests beside the working tree's, read once per
run and cached under the commit. A check reads them, derives what its
section leaves out, and emits its `derived:` and `pinned:` lines through the
sink, above its own row. The runner prints none of its own. Planning a run
asks each catalogue row's `available` fact whether the tree holds what an
Automatic check applies to, and derives no number to answer. `--list` prints
one `pinned:` line per value a section states and derives nothing.

**`init` writes the opt-in marker, and `--pin` writes guardrails.** Plain
`init` writes `{}` and reads no tree. `init --pin` writes today's complexity
ceilings, a ceiling per document the derivation commit holds and the radius
history gives, each only where the configuration states none, so a pinned
number, a schedule, a `false`, the accepted list and the journal preference
stay as a person wrote them. It never writes roots, languages, document
entries, manifests, test roots, families or build commands. `--add` and
`--force` are gone with the snapshot they wrote.

## What this is not

No registry replaces the removed sections, no trait abstracts a check's
policy, and no persistent project cache or package graph is added. The facts
stay one lazily computed value per run. `reachability` keeps its private,
cached family representation, which is measurement state and never
configuration.

## Consequences

- A repository opts in with `{}` and changes nothing as it grows: a new
  document, test directory, manifest or package is gated on the run that
  finds it.
- A person reads `klin.json` as a list of decisions. A value in it is one
  somebody chose, and a value missing from it is printed with its rule.
- `was_held` reads the facts whichever gates run, so the 7.1 rule for an
  unheld root no longer depends on which other sections a run derived.
- Pre-compact configurations fail loudly on first load, naming each retired
  field and its replacement.
- Discovering the facts costs about what a fully pinned configuration saved.
  Measured on 2026-09-14, macos/aarch64, release builds, median of five, with
  the binary of 997664d beside this change. The `legacy` column is the earlier
  binary under the configuration its own `init --force` pinned, with the build
  off; `build-off` is `{"build": []}`; `{}` derives the build through stand-in
  `cargo` and `tsc` commands, so it measures the build's preparation and two
  process starts, not a compiler:

  | Row | legacy, before | build-off, before | build-off, after | `{}`, before | `{}`, after |
  |---|---|---|---|---|---|
  | 2k warm hook | 1,631 | 1,494 | 1,535 | 2,026 | 1,973 |
  | 2k cold survey | 1,994 | 1,829 | 1,902 | 1,873 | 1,846 |
  | 2k strict | 1,222 | 1,223 | 1,227 | 1,243 | 1,225 |
  | 10k warm hook | 6,401 | 5,957 | 6,042 | 6,804 | 6,964 |
  | 10k cold survey | 13,829 | 13,756 | 14,191 | 14,161 | 14,244 |
  | 10k strict | 5,620 | 5,860 | 5,752 | 5,783 | 5,975 |

  The 2,000-file source-area strict rows were 1,115, 1,347 and 1,469 ms before
  and 1,069, 1,282 and 1,417 ms after for 2, 100 and 500 areas. The 300k dense
  row was 15,789, 28,505 and 17,714 ms before and 15,802, 28,809 and 17,654 ms
  after for warm hook, cold survey and strict. This repository's own
  `klin gate --strict`, under its full configuration and the earlier binary,
  took 2,045 ms, and under the compact configuration and this binary 1,995 ms.
  No row moved by more than a third; the largest moves are within 4%.

  Issue #181 was measured on 2026-09-18, macos/aarch64, release builds, with
  the same #157 fixture and five samples per row. The parent binary was
  `091a5e0`; the schema binary was `bc8dd41`. Both used `{"build": []}` and
  the hook rows exclude the project's build command:

  | Row | Before (ms) | After (ms) | Change |
  |---|---:|---:|---:|
  | 2k warm hook | 306 | 346 | +13.1% |
  | 2k cold survey | 1,934 | 2,161 | +11.7% |
  | 2k strict | 1,240 | 1,287 | +3.8% |
  | 10k warm hook | 475 | 503 | +5.9% |
  | 10k cold survey | 15,192 | 15,538 | +2.3% |
  | 10k strict | 6,014 | 6,082 | +1.1% |
  | guard, 1,000 events | 11,148 | 11,650 | +4.5% |

  The schema path is not read by gate, hook or guard. The binary carries the
  generator for the explicit `reference --schema` command, while the measured
  runtime paths remain unchanged; no performance-specific implementation is
  justified by these medians.

  The final rebased issue-181 tree was remeasured on 2026-09-19 against
  `origin/main` at `f9519fd`, on the same macos/aarch64 machine and release
  toolchain. The existing fixture was run for five samples per row with
  `{"build": []}`; its counters and digests matched the base in every case:

  | Row | origin/main (ms) | final (ms) | Change |
  |---|---:|---:|---:|
  | 2k warm hook | 300 | 297 | -1.0% |
  | 2k cold survey | 1,894 | 1,869 | -1.3% |
  | 2k strict | 1,194 | 1,191 | -0.3% |
  | 10k warm hook | 483 | 487 | +0.8% |
  | 10k cold survey | 14,447 | 14,343 | -0.7% |
  | 10k strict | 5,702 | 5,626 | -1.3% |
  | guard, 1,000 events | 10,375 | 10,450 | +0.7% |

  The release binary was 22,224,528 bytes at `origin/main` and 22,306,768
  bytes in the final tree (+82,240 bytes, +0.37%). That increase is negligible,
  so the deterministic generator remains in the binary; no performance
  optimization or feature split is justified.

  The same final-head comparison covered the #175 source-dense fixtures. Both
  fixtures kept their exact counters and digest. The 1M row was stable within
  normal run variance:

  | Row | origin/main (ms) | final (ms) | Change |
  |---|---:|---:|---:|
  | 300k warm hook | 776 | 768 | -1.0% |
  | 300k cold survey | 25,977 | 56,319 | +116.8% |
  | 300k strict | 13,967 | 29,587 | +111.8% |
  | 1M warm hook | 1,989 | 2,105 | +5.8% |
  | 1M cold survey | 96,008 | 95,322 | -0.7% |
  | 1M strict | 64,252 | 68,187 | +6.1% |

  The 300k final row was repeated twice, and the base was rerun between them.
  In complete observation order, final medians were
  `768/56,319/29,587`, `1,081/43,415/23,864` and
  `1,300/43,171/22,764`; base medians were `776/25,977/13,967` and
  `879/31,846/22,053` (warm/cold/strict). The changed medians move with host
  state rather than with the schema code (the 1M rows and the 2k/10k rows stay
  within 6.1%), so this is recorded as benchmark noise and does not justify a
  hot-path change. The controlled #193 dense measurements remain the release
  baseline.
