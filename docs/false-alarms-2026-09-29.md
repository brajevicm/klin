# False alarms of the default configuration, 2026-09-29

This document belongs to #343. When `klin.json` is `{}`, how many of the
failures klin prints on ordinary commits of real repositories would a person
call appropriate? It also labels the failing stops in klin's own
journal the same way. The labels then decide three defaults: whether
`public-api` is on or opt-in, which documents `doc-size` judges, and the
complexity floors.

It is not a benchmark, and it supports no claim about effectiveness. The sample
is ten repositories that the rules below pick, and the result is a count of
labeled failures on that sample.

## Rules, written before any run

The rules in this section were committed before klin ran on any repository of
the sample, and the commit that adds this section records that order.

### Repositories

1. Run one GitHub repository search per language on 2026-09-29, with this
   query, sorted by stars in descending order, 30 results per page, first page
   only:

   ```text
   language:<Rust|TypeScript> stars:1000..20000 pushed:>=2026-09-15 archived:false mirror:false size:<=150000
   ```

   The star band leaves out toy repositories and the largest monorepos. The
   size limit, in kilobytes, keeps a full clone practical. The push date keeps
   repositories that still take commits. GitHub leaves forks out of a search by
   default.
2. Keep the two responses as they came, in the evidence directory.
3. Walk each list in its order. A repository is eligible when all of these
   hold:
   - A full clone of its default branch finishes.
   - The start commit (next section) holds `Cargo.toml` at the root for Rust,
     or `package.json` at the root for TypeScript.
   - The start commit holds no `klin.json` at the root, because the replay
     writes its own.
   - The first-parent walk from the start commit reaches ten changes.
4. Take the first five eligible repositories of each list. Record every
   repository skipped before them, with the rule it failed.

### Commits

1. The start commit is the first commit on the first-parent walk of the default
   branch whose committer date is earlier than 2026-09-29T00:00:00Z.
2. The ten changes of a repository are the start commit and the nine commits
   before it on that walk. Each change has its first parent as the base and the
   commit as the head.
3. No commit is left out for what it touches. A release, a formatting commit, a
   dependency bump or a documentation change stays in the sample. A merge
   commit on the first-parent walk stays too, and its change is everything it
   merged.
4. The selection is frozen in `selection.json` with each commit id. A later run
   reads that file and needs no search.

### The run

1. klin is the release binary that `benchmark/build-klin` builds from this
   branch. The branch changes no file under `src/`, so the binary is klin at
   `main` 4c2da5dd. Its provenance file goes into the evidence directory.
2. For each change, in a full clone with its remote removed:
   - every local branch is deleted, `main` is set to the base, and a branch
     named `change` is checked out at the head;
   - the tree is reset and cleaned, and an untracked `klin.json` holding `{}`
     is written at the root;
   - `klin gate --json` runs at the root, with `GITHUB_BASE_REF` and
     `GITHUB_EVENT_PATH` removed from the environment, and a limit of 600
     seconds.
3. That is the branch window of SPEC 6.3, with the base as `before`, as #289
   ran it. The Action adds `--strict`. A failure that only `--strict` adds is
   outside this sample.
4. Each run keeps its exit status, its standard output and error, and its wall
   time.

### What gets a label

1. One worksheet row for each gate that a run reports as FAIL or as a tool
   error. A run that exits 2 before any gate runs is one row too.
2. A row shows what a person needs to judge it at the moment it fired: the
   repository, the commit message, the files the change touched, the gate, the
   derived values klin printed for that gate, every finding with its values and
   its base match, a code excerpt from the head tree, and the remedy klin
   printed.
3. The label is `appropriate` or `not-appropriate`, with an optional note. A
   person gives it. The agent that builds the worksheet gives none. A row whose
   findings split between the two takes the label of the finding that most
   wants a person's attention, and the note names the others.
4. The labels are stored in one file keyed by row id, and its SHA-256 is
   recorded before any summary reads it.

### klin's own journal

1. The population is every stop in `.git/klin/journal.jsonl` whose status is
   FAIL or ERROR, as the file stands on 2026-09-29.
2. One row for each gate that such a stop reports as failed. Consecutive stops
   of one session whose findings for that gate are the same collapse into one
   row, with the number of stops it stands for.
3. A row shows the session's last prompt before the stop, the klin version, the
   window, every finding with its values, and the remedy. The tree the stop
   judged is gone, so a row holds no code excerpt beyond the finding's own
   line text.
4. The labels are the same two, stored and hashed the same way.

### What the labels decide

A person decides each default from the labeled rows of its gate, over both
populations. The worksheet groups the rows so each decision reads its own:

- `public-api`: every row of the gate.
- `doc-size`: the rows by document name.
- complexity floors: the rows by whether the ceiling that fired was the floor,
  `cc 5` or `lines 25`, or a derived percentile.

## Sample

The two searches ran on 2026-09-29. Their responses are
`benchmark/evidence/false-alarms-2026-09-29/search-rust.json` and
`search-typescript.json`, and `selection.json` beside them freezes the result.
The Rust list skipped one repository before it reached five:
`ccusage/ccusage`, whose start commit holds no `Cargo.toml` at the root. The
TypeScript list skipped none.

| Repository | Language | Stars | Size (MB) | Functions at the newest base | Derived complexity ceiling |
|---|---|---|---|---|---|
| `firecrawl/pdf-inspector` | Rust | 19409 | 15 | 4,348 | cc 13, lines 80 |
| `gfx-rs/wgpu` | Rust | 18156 | 87 | 8,886 | cc 15, lines 107 |
| `louis-e/arnis` | Rust | 18096 | 122 | 13,674 | cc 13, lines 51 |
| `eythaann/Seelen-UI` | Rust | 17899 | 92 | 4,177 | cc 9, lines 44 |
| `denisidoro/navi` | Rust | 17687 | 2 | 184 | cc 9, lines 45 |
| `refactoringhq/tolaria` | TypeScript | 19930 | 95 | 34,768 | cc 5, lines 34 |
| `whyour/qinglong` | TypeScript | 19912 | 19 | 5,056 | cc 7, lines 57 |
| `apollographql/apollo-client` | TypeScript | 19804 | 103 | 13,269 | cc 5, lines 108 |
| `Open-Dev-Society/OpenStock` | TypeScript | 19463 | 2 | 642 | cc 6, lines 49 |
| `mountain-loop/yaak` | TypeScript | 19260 | 49 | 8,495 | cc 9, lines 52 |

Function counts and ceilings come from the complexity `derived:` line of each
repository's newest change, whose base is the parent of the start commit. For
`refactoringhq/tolaria` and `apollographql/apollo-client`, the 95th percentile
of `cc` fell below 5, so the floor set their `cc` ceiling. In every run the
derived `lines` ceiling was 34 or more. With no repository under 50 functions, the
replay never reached the `lines 25` floor, and #289 remains the only evidence on
it.

## Runs

The binary is `klin 0.3.0`, which `benchmark/build-klin` built from `8a75dcb2`,
SHA-256 `a26ea43387e5e0998199e5c7a1d5b227d9645e0ca994aec044e837238f0c2c17`.
`8a75dcb2` changes nothing under `src/` against `main` at 4c2da5dd. Each run is
one file under `runs/`. Each run started with no `.git/klin` state in its
clone. Every run finished inside the limit and printed a JSON report.

| Repository | PASS | FAIL | ERROR | Other exit | Median klin time (s) |
|---|---|---|---|---|---|
| `firecrawl/pdf-inspector` | 3 | 7 | 0 | 0 | 11.9 |
| `gfx-rs/wgpu` | 7 | 3 | 0 | 0 | 30.3 |
| `louis-e/arnis` | 1 | 9 | 0 | 0 | 20.8 |
| `eythaann/Seelen-UI` | 10 | 0 | 0 | 0 | 6.4 |
| `denisidoro/navi` | 6 | 4 | 0 | 0 | 0.6 |
| `refactoringhq/tolaria` | 8 | 2 | 0 | 0 | 21.7 |
| `whyour/qinglong` | 5 | 5 | 0 | 0 | 3.5 |
| `apollographql/apollo-client` | 7 | 2 | 1 | 0 | 16.6 |
| `Open-Dev-Society/OpenStock` | 4 | 5 | 1 | 0 | 0.9 |
| `mountain-loop/yaak` | 2 | 8 | 0 | 0 | 9.9 |
| all | 53 | 45 | 2 | 0 | |

The two ERROR runs exit 2. In `apollographql/apollo-client` change 7
(`0c925a4348`), the TypeScript grammar rejected
`src/utilities/internal/types/Exact.ts`, and `public-api` could not resolve five
`export declare namespace` forms. In `Open-Dev-Society/OpenStock` change 9
(`3cf500ad4c`), the grammar rejected `lib/constants.ts` and `lib/markets.ts`.

## Rows

The replay gives 79 rows over the 47 runs that did not pass. The journal, as it
stood on 2026-09-29, holds 158 failing stops: 151 FAIL and 7 ERROR, with klin
versions from 0.1.0 to 0.3.0. When #343 was written it held 140 failing stops,
103 of them for inventory; now 109 of the 158 fail inventory. The 158 stops give
212 gate occurrences, which collapse into 138 journal rows. The copy the
worksheet read has the SHA-256
`dfd82784a2dd3e8b623071d8e92481c83e3347afa602e6b793019d72f44ed0af`.

| Gate | Replay rows | Journal rows | Journal stop occurrences |
|---|---|---|---|
| complexity | 37 | 27 | 41 |
| dead-symbols | 8 | 4 | 13 |
| doc-citations | 0 | 2 | 2 |
| doc-size | 16 | 4 | 6 |
| escapes | 13 | 17 | 29 |
| inventory | 0 | 76 | 109 |
| layering | 0 | 5 | 9 |
| public-api | 2 | 0 | 0 |
| reachability | 1 | 0 | 0 |
| run | 0 | 1 | 1 |
| stubs | 2 | 2 | 2 |
| all | 79 | 138 | 212 |

Before any label, three facts limit what the rows can decide:

- `public-api`: two replay rows, R013 in `gfx-rs/wgpu` (FAIL) and R057 in
  `apollographql/apollo-client` (ERROR), and no journal row.
- Complexity: 35 replay complexity rows include a site the base already held
  over its ceiling that grew. Four rows include a `cc` finding at the floor,
  two in `refactoringhq/tolaria` and two in `apollographql/apollo-client`.
  No row reaches the `lines` floor. Every journal complexity row runs under
  the ceiling that klin's own `klin.json` pins, `cc 8` and `lines 60`, so the
  journal says nothing about the floors.
- `doc-size`: of the 16 replay rows, 10 include `CHANGELOG.md`, 5 include
  `README.md`, and one each includes `MARKET_SUPPORT.md` and `README-en.md`.
  One row fails on two documents. Three of the four journal `doc-size` rows
  judge a ceiling that klin's own `klin.json` pinned at the time. J051 judges
  the derived ceiling of `RELEASE_NOTES.md`.

## Changes to the rules after the first run

These change what a row shows, and one adds a row the rules did not name:

- A journal row also shows the first prompt of its session, because the last
  prompt is often only `<task-notification>`.
- A row shows code excerpts for its first three findings only, each at most 40
  lines, a commit message of at most 30 lines, at most 30 touched files, and
  prompts of at most 600 characters. One replay row holds 167 findings, and
  with an excerpt for each the worksheet was 51,613 lines long.
- An ERROR stop whose build failed reports no gate, so it becomes one row with
  the gate `run`, as a replay run that exits 2 before any gate would. The
  journal has one such row.

## Labels

Labels are pending. A person labels every row in
`benchmark/evidence/false-alarms-2026-09-29/labels.json`. The replay rows are in
`worksheet.md` and the journal rows in `journal-worksheet.md`. Then:

1. Record the SHA-256 of `labels.json` in this section.
2. Fill the label column of the table below, and count the labels by gate and
   by decision group.
3. Record the default for `public-api`, `doc-size` and the complexity floors
   in #207.

| Row | Repository | Commit | Gate | Decision group | Findings | Label |
|---|---|---|---|---|---|---|
| R001 | `firecrawl/pdf-inspector` | `bf6800920a` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R002 | `firecrawl/pdf-inspector` | `bf6800920a` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 2 | pending |
| R003 | `firecrawl/pdf-inspector` | `3400b4725e` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R004 | `firecrawl/pdf-inspector` | `3400b4725e` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 1 | pending |
| R005 | `firecrawl/pdf-inspector` | `ac275044ed` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R006 | `firecrawl/pdf-inspector` | `ac275044ed` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 2 | pending |
| R007 | `firecrawl/pdf-inspector` | `e1797b1f48` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R008 | `firecrawl/pdf-inspector` | `e1797b1f48` | complexity FAIL | cc at a derived percentile; lines at a derived percentile; lines at a derived percentile, the base site already over | 6 | pending |
| R009 | `firecrawl/pdf-inspector` | `9a055d5f3c` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R010 | `firecrawl/pdf-inspector` | `9a055d5f3c` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 5 | pending |
| R011 | `firecrawl/pdf-inspector` | `2dbd16b3ea` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R012 | `firecrawl/pdf-inspector` | `f06bb34075` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R013 | `gfx-rs/wgpu` | `fcd059f423` | public-api FAIL | public-api | 3 | pending |
| R014 | `gfx-rs/wgpu` | `5a7601baf4` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R015 | `gfx-rs/wgpu` | `5a7601baf4` | escapes FAIL | escapes | 1 | pending |
| R016 | `gfx-rs/wgpu` | `5a7601baf4` | complexity FAIL | cc at a derived percentile, the base site already over | 1 | pending |
| R017 | `gfx-rs/wgpu` | `2c3e503258` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 2 | pending |
| R018 | `louis-e/arnis` | `daee5a7bff` | escapes FAIL | escapes | 2 | pending |
| R019 | `louis-e/arnis` | `daee5a7bff` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 19 | pending |
| R020 | `louis-e/arnis` | `d4852aa3d7` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 28 | pending |
| R021 | `louis-e/arnis` | `d4852aa3d7` | dead-symbols FAIL | dead-symbols | 2 | pending |
| R022 | `louis-e/arnis` | `641c2100df` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 3 | pending |
| R023 | `louis-e/arnis` | `7e9a4344c5` | doc-size FAIL | document README.md | 1 | pending |
| R024 | `louis-e/arnis` | `7e9a4344c5` | escapes FAIL | escapes | 4 | pending |
| R025 | `louis-e/arnis` | `7e9a4344c5` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 29 | pending |
| R026 | `louis-e/arnis` | `69811f59d2` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 8 | pending |
| R027 | `louis-e/arnis` | `788d5e9ca9` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 9 | pending |
| R028 | `louis-e/arnis` | `e7cdeb99e2` | escapes FAIL | escapes | 12 | pending |
| R029 | `louis-e/arnis` | `e7cdeb99e2` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 35 | pending |
| R030 | `louis-e/arnis` | `fc19146b3f` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 8 | pending |
| R031 | `louis-e/arnis` | `fc19146b3f` | dead-symbols FAIL | dead-symbols | 1 | pending |
| R032 | `louis-e/arnis` | `80553b383f` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 28 | pending |
| R033 | `denisidoro/navi` | `a171c2938d` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 1 | pending |
| R034 | `denisidoro/navi` | `9e6e8da6f5` | escapes FAIL | escapes | 3 | pending |
| R035 | `denisidoro/navi` | `cc40723617` | stubs FAIL | stubs | 1 | pending |
| R036 | `denisidoro/navi` | `75eb060927` | escapes FAIL | escapes | 2 | pending |
| R037 | `refactoringhq/tolaria` | `04030c3e0a` | complexity FAIL | cc at the floor, the base site already over; lines at a derived percentile, the base site already over | 1 | pending |
| R038 | `refactoringhq/tolaria` | `44740e4ae1` | complexity FAIL | cc at the floor, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 3 | pending |
| R039 | `refactoringhq/tolaria` | `44740e4ae1` | dead-symbols FAIL | dead-symbols | 1 | pending |
| R040 | `whyour/qinglong` | `44129ca088` | complexity FAIL | lines at a derived percentile; lines at a derived percentile, the base site already over | 2 | pending |
| R041 | `whyour/qinglong` | `bc0f35e3f7` | escapes FAIL | escapes | 1 | pending |
| R042 | `whyour/qinglong` | `bc0f35e3f7` | complexity FAIL | lines at a derived percentile, the base site already over | 1 | pending |
| R043 | `whyour/qinglong` | `f168efcdae` | escapes FAIL | escapes | 1 | pending |
| R044 | `whyour/qinglong` | `f168efcdae` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 4 | pending |
| R045 | `whyour/qinglong` | `6c487018c2` | complexity FAIL | cc at a derived percentile; lines at a derived percentile; lines at a derived percentile, the base site already over | 3 | pending |
| R046 | `whyour/qinglong` | `801a71d740` | doc-size FAIL | document README-en.md; document README.md | 2 | pending |
| R047 | `whyour/qinglong` | `801a71d740` | escapes FAIL | escapes | 8 | pending |
| R048 | `whyour/qinglong` | `801a71d740` | complexity FAIL | cc at a derived percentile; lines at a derived percentile | 167 | pending |
| R049 | `apollographql/apollo-client` | `37f700eb4c` | complexity FAIL | cc at the floor; lines at a derived percentile, the base site already over | 2 | pending |
| R050 | `apollographql/apollo-client` | `d4f8770120` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R051 | `apollographql/apollo-client` | `0c925a4348` | doc-size FAIL | document CHANGELOG.md | 1 | pending |
| R052 | `apollographql/apollo-client` | `0c925a4348` | escapes FAIL | escapes | 150 | pending |
| R053 | `apollographql/apollo-client` | `0c925a4348` | stubs FAIL | stubs | 6 | pending |
| R054 | `apollographql/apollo-client` | `0c925a4348` | complexity ERR | cc at the floor; cc at the floor, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over; unparsed record | 149 | pending |
| R055 | `apollographql/apollo-client` | `0c925a4348` | dead-symbols ERR | dead-symbols | 2 | pending |
| R056 | `apollographql/apollo-client` | `0c925a4348` | reachability FAIL | reachability | 3 | pending |
| R057 | `apollographql/apollo-client` | `0c925a4348` | public-api ERR | public-api | 26 | pending |
| R058 | `Open-Dev-Society/OpenStock` | `391e72d780` | escapes FAIL | escapes | 1 | pending |
| R059 | `Open-Dev-Society/OpenStock` | `87df76a2ce` | doc-size FAIL | document README.md | 1 | pending |
| R060 | `Open-Dev-Society/OpenStock` | `87df76a2ce` | complexity FAIL | lines at a derived percentile, the base site already over | 1 | pending |
| R061 | `Open-Dev-Society/OpenStock` | `130a69734d` | doc-size FAIL | document MARKET_SUPPORT.md | 1 | pending |
| R062 | `Open-Dev-Society/OpenStock` | `130a69734d` | complexity FAIL | lines at a derived percentile | 1 | pending |
| R063 | `Open-Dev-Society/OpenStock` | `e844ac413c` | doc-size FAIL | document README.md | 1 | pending |
| R064 | `Open-Dev-Society/OpenStock` | `e844ac413c` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 4 | pending |
| R065 | `Open-Dev-Society/OpenStock` | `75a9ebb6f5` | complexity FAIL | cc at a derived percentile, the base site already over | 1 | pending |
| R066 | `Open-Dev-Society/OpenStock` | `3cf500ad4c` | doc-size FAIL | document README.md | 1 | pending |
| R067 | `Open-Dev-Society/OpenStock` | `3cf500ad4c` | escapes FAIL | escapes | 3 | pending |
| R068 | `Open-Dev-Society/OpenStock` | `3cf500ad4c` | complexity ERR | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over; unparsed record | 30 | pending |
| R069 | `Open-Dev-Society/OpenStock` | `3cf500ad4c` | dead-symbols ERR | dead-symbols | 6 | pending |
| R070 | `mountain-loop/yaak` | `7f30253616` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 2 | pending |
| R071 | `mountain-loop/yaak` | `6c2a18d506` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 2 | pending |
| R072 | `mountain-loop/yaak` | `0a57d8eb61` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over | 4 | pending |
| R073 | `mountain-loop/yaak` | `411aa262c7` | dead-symbols FAIL | dead-symbols | 2 | pending |
| R074 | `mountain-loop/yaak` | `34815f327a` | escapes FAIL | escapes | 4 | pending |
| R075 | `mountain-loop/yaak` | `34815f327a` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 12 | pending |
| R076 | `mountain-loop/yaak` | `34815f327a` | dead-symbols FAIL | dead-symbols | 1 | pending |
| R077 | `mountain-loop/yaak` | `6fc43a60e2` | complexity FAIL | cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over | 4 | pending |
| R078 | `mountain-loop/yaak` | `23d369d84e` | dead-symbols FAIL | dead-symbols | 1 | pending |
| R079 | `mountain-loop/yaak` | `b03c41b767` | complexity FAIL | cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over | 3 | pending |
