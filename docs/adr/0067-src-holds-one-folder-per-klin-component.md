# `src/` holds one folder per klin component

> Completes ADR 0065 rule 8. Recorded for #600, 2026-10-10. Paths and line
> numbers are at `80188ef5`. The research, the four first layouts and the two
> adversarial reviews are in #599.

ADR 0065 rule 8 says that broad decomposition and final directory
normalization wait until `klin layering` reports zero cycles. #427 reached
zero at `748d01fc`, and #370 is closed. This ADR records that last step.

At `80188ef5`, 55 modules sit at the root of `src/` beside `main.rs`. Their
names mix domain terms (`turn`, `radius`, `guard`), mechanisms (`cache`,
`write`, `key`, `record`) and checks (`complexity`, `escapes`). Command names
do not match file names: `klin report` is in `stats.rs`, `klin setup` is in
`hooks.rs`, and `klin check` and the Stop are both in `gate.rs`. The
`layering` section of `klin.json` names 60 paths in 14 layers, and its `core`
layer holds 20 of them.

## The decision

**`src/` holds `main.rs` and twelve folders. Each folder is one klin
component, named by its `CONTEXT.md` or SPEC 3.1 term where one exists, and
each folder is one layer of `klin.json`.**

From the bottom layer to the top. The names are the module names after #602
and #603. #602 holds the mapping from each file of today to its new path.

| Folder | Holds | Term |
| --- | --- | --- |
| `sys/` | `git`, `shell`, `write`, `state`, `cache`, `error`, `record`, `changed`, `hunks`, `clock` | no klin concept, and the one git seam (ADR 0041) |
| `config/` | `file`, `key`, `ceiling`, `scope` | Config, Ceiling, Scope, Pin |
| `syntax/` | the parser and the structural facts | the parser seam (ADR 0035) |
| `facts/` | `tree`, `files`, `survey` | Tree, SPEC 3.1 Facts |
| `modules/` | the module graph and its resolvers | ADR 0043 |
| `surface/` | the public surfaces | ADR 0044 |
| `window/` | `stamp`, `base` | Window, Base |
| `contract/` | `check`, `holes`, `measurement`, `ratchet`, `coverage`, `project` | what a check gets and returns, and Project (ADR 0038) |
| `checks/` | one file per check, and `structural/` for `layering`, `reachability`, `dead_symbols` and `public_api` | Check, SPEC 9.1 |
| `engine/` | `catalogue`, `plan`, `document`, `diagnostics`, `render`, `reference`, `against` | SPEC 3.1 Catalogue and Renderers, and the part of the Engine that runs the checks |
| `hook/` | `stop`, `stats`, `budget`, `build`, `guard`, `handoff`, `journal`, `radius`, `turn`, `host/` | the host protocol, Guard, Journal, Turn, Radius (SPEC 10) |
| `cli/` | one file per command: `agent`, `check`, `setup`, `status`, `report`, `policy`, `update`, and `starter`, the `klin.json` writer of `setup` | SPEC 11, and the hidden ingress of SPEC 10.1 |

`engine/` holds only the part of SPEC 3.1's Engine that runs the checks.
The code that measures is in `checks/`, and the ratchet is in `contract/`.

### Rules

1. Each layer is one folder. A layer may use every layer below it in the
   table. A dependency on a layer above it fails the `layering` gate, and
   `acyclic: true` fails a cycle.
2. `cli/` holds the entry points, one file per command. No file outside
   `cli/` holds a clap type. Logic that another entry point needs lives in a
   layer below `cli/`.
3. The `in` of each layer is its folder, and the `cli` layer also holds
   `src/main.rs`. A new file in an existing folder needs no change to
   `klin.json`. A new folder is a new layer: only the owner commits it, and it
   amends this ADR.
4. A new file goes in the folder of the component it serves, or in `sys/`
   when it holds no klin concept. That folder must sit above every layer the
   file uses. When no folder fits, a dependency points the wrong way: move
   the code that the file needs down, as #601 does.
5. The `mod.rs` of a new folder holds only `mod` lines, as `src/check/mod.rs`
   does at `80188ef5`. The `mod.rs` files of `syntax/`,
   `syntax/structural/`, `modules/`, `surface/` and `hook/host/` keep their
   code. Containment is not an edge (ADR 0043), so a child that uses its
   parent, as `src/syntax/pattern.rs:320` uses `syntax::language_of`, makes
   no cycle while the parent does not use that child.
6. No module has the name of the folder that holds it. clippy's
   `module_inception` lint fails under the `-D warnings` that CI passes
   (`.github/workflows/quality.yml:108`). A scratch crate confirmed it for
   `contract/contract.rs`.
7. Language-specific code lives in its seam, beside the language-agnostic
   code of that seam. `syntax/structural/`, `modules/` and `surface/` each
   hold one file per language. A language that gets structural support adds
   one file to each. A check keeps its own per-language rule rows, because
   they are the check's own policy (ADR 0035, `src/complexity.rs:110`).
8. A person reads which languages each check supports in `docs/REFERENCE.md`.
   A CLI test pins that file to the binary's output, so it cannot drift. A
   file name is no proof of support.
9. A move adds no re-export (ADR 0065 rule 3).
10. A pure move and a split of a file land in separate pull requests. A pure
    rename keeps each file above git's `-M50%` rename limit, so held findings
    keep their sites (spec 7.2).

### Placements that are not obvious

Each of these files sits where it does because of a dependency. To move it
to the folder its name suggests brings back an upward edge.

- `project` is in `contract/`, not `facts/`. `Project` reads the stamp and
  the base (`src/project.rs:23`), so it sits above `window/`, and every check
  borrows it.
- `stats` is in `hook/`. The Stop calls `stats::stop_tail` and
  `stats::turn_end` for its turn-end recap (`src/gate.rs:412-414`). The
  `klin report` command goes to `cli/report.rs` (#603).
- `catalogue` is in `engine/`. Its rows name the check functions, so it sits
  above `checks/`, although SPEC 3.2 lists the catalogue in the policy layer.
- `render` is in `engine/`, because `document` calls it
  (`src/document.rs:21`).
- `against` is in `engine/`. It is the window choice that both paths call
  (SPEC 6.1), and it takes the base and the `Project`. Its name follows the
  code's `Against` type and avoids a second "window".
- `changed` and `hunks` are in `sys/`. They read git output and import only
  `git` and `error`, and `changed` is one of the two files that the git seam
  excepts (ADR 0041).
- `stamp` and `base` are in `window/`, the two inputs of a Window. The
  stamp also answers which findings a blocked Stop already asked about
  (#601).
- `init.rs` becomes `cli/starter.rs`. It writes the `klin.json` of `setup`,
  and it imports the catalogue, `complexity`, `radius` and `Project`, so it
  sits above `hook/`.
- `gate.rs` moves whole to `hook/stop.rs`, and #603 then splits it three
  ways: `cli/check.rs`, `engine/against.rs` and `hook/stop.rs`.

The order of the layers is in `klin.json`, and #602 adds a code map with the
same order to `CONTRIBUTING.md`. A folder listing sorts by name, so it cannot
show the order.

## Known costs

- Most `use crate::` lines change one time, in the move of #602. The paths
  into `syntax/`, `modules/` and `surface/` stay. Each branch that is open at
  that time must rebase across the move.
- A module that moved from the root of `src/` gets one more path segment, for
  example `crate::sys::git` in place of `crate::git`.
- The structural cache checksums its own source files
  (`src/syntax/structural/cache.rs:29-38`). The move changes their `use`
  lines, so every cached base is built again one time after the move.
- The layering does not show which languages a check supports. Rule 8 puts
  that in the reference.

## Rejected

- **One folder per lane of #590**, as an inverse Conway maneuver over the
  agent sessions. #590 is a roadmap, so its lanes change. Of the 489
  non-merge commits up to `80188ef5` that change a file that still exists
  there, 269 change files in two or more of the twelve folders. Git conflicts
  occur per file and hunk, not per folder, and worktrees keep sessions apart
  (#590 rule 3).
- **One `languages/<lang>/` folder per language.** Of the 26 commits that
  changed `syntax/structural/rust.rs` up to `80188ef5`, 18 also changed
  `typescript.rs`, so a change goes across the languages of one seam. To
  move a check's rows out of the check contradicts ADR 0035. It also turns
  held sites into new findings: `SLASH` at `src/stubs.rs:25` matches its own
  row, and `escapes.rs` would fall below `-M50%`.
- **Group folders above the twelve** (`integration/`, `measurement/`,
  `policy/`, `platform/`). They show the layers in one listing, but they add
  a segment to every path, and a reader must classify a component before
  they can find it. They also claim the SPEC 3.2 groups, which the code does
  not follow: the catalogue names check functions, and `document.rs` calls
  the renderer.
- **One folder per SPEC 3.1 component**, with `ratchet/`, `guard/` and
  `journal/` apart. Several folders would hold one file each.
- **A Cargo workspace with flat `crates/`**, as in matklad's "Large Rust
  Workspaces". `public-api` treats a sibling crate as external (`CONTEXT.md`
  "External", ADR 0044), so every item that crosses a crate would become
  ratcheted surface. The `layering` gate already
  enforces the boundaries.
- **A language registry that every seam imports.** A language file that
  reaches a seam table through any path closes a cycle. One such path exists
  at `80188ef5`: `src/modules/rust.rs:160` calls `survey::surveyed`,
  `survey.rs` imports `tree`, and `src/tree.rs:5` imports
  `syntax::structural::Extracted`. Self-registration
  (`linkme`, `inventory`) or a build script would hide the wiring from klin's
  own `layering` and `reachability`. #606 holds the deferred design that
  avoids the cycle.

## Consequences

- #601 removes the two edges that point upward under this layering:
  `inventory.rs` to `turn`, and the engine files to `journal::timed` and
  `journal::millis`.
- #602 moves the files in one pull request of pure renames, with the new
  `layering` section that the owner commits.
- #603 gives each command its own file in `cli/`. It splits `gate.rs` into
  `cli/check.rs`, `engine/against.rs` and `hook/stop.rs`, and `stats.rs`
  into `cli/report.rs` and `hook/stats.rs`.
- #604 adds a language-by-check matrix to `docs/REFERENCE.md`.
- #605 decides whether the seven lists of languages become one table, and
  records the decision as an amendment to this ADR.
- #606 holds the language descriptor and its composition root, for when a
  third language gets structural support or at the next performance round,
  whichever comes first.
- This ADR changes no behavior and no SPEC rule.
