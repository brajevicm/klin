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
