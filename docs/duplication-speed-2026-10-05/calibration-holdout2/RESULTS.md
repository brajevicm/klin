# Third label set: version 2 rules (#480)

This note confirms the version 2 rules on labels that the rules never saw.
Version 2 was chosen after `calibration-holdout/` was read, so that set is
not evidence for it.

## Method

1. Atuin (Rust) and documenso (TypeScript) were the fallback candidates in the
   owner-approved proposal. `provenance.json` records the pinned commits.
2. `calibrate.py collect` found 2,374 matches with no repeated pair id: 144
   Rust pairs and 2,230 TypeScript pairs.
3. `holdout.py rules` wrote `rules.json` before any label existed. Commit
   9bb508ef froze it.
4. Four blind model reviewers labeled four shards. They kept scratch files in
   their own directories. An API limit stopped all four once. Each resumed
   with its own context. No pair id received two different labels.

Version 2 rules:

- Both languages: no pair whose two spans overlap in one file.
- Rust: E2 (imports and attributes trimmed) and E3 (no pair with both spans in
  trait impls), T = 100.
- TypeScript: at least 50% of each span inside complete functions, at least 60
  tokens outside JSX, T = 60.

## Result

| Language | Kept | Copy | Precision | Limit 80% | Copies kept / all copies | Precision of all pairs at T |
|---|---:|---:|---:|---|---:|---:|
| Rust (atuin) | 26 | 26 | 100% | met | 26 / 105 | 27/28 = 96.4% |
| TypeScript (documenso) | 140 | 128 | 91.4% | met | 128 / 880 | 880/2,230 = 39.5% |

TypeScript non-copies that the rule kept: 10 boilerplate and 2
required-shape. Most are pairs of sibling dialogs and tables, for example
create and edit webhook dialogs and team and organisation member tables.

## Version 2 across all three label sets

| Set | Rust | TypeScript |
|---|---:|---:|
| `calibration/` (rules were designed here) | 22/26 = 84.6% | 23/23 = 100% |
| `calibration-holdout/` (version 2 was designed here) | 53/59 = 89.8% | 89/93 = 95.7% |
| `calibration-holdout2/` (blind confirmation) | 26/26 = 100% | 128/140 = 91.4% |

## Limits of this evidence

- The Rust confirmation is weak. Atuin gives only 28 Rust pairs at T >= 100,
  and 27 of them are copies without any rule. This set cannot show that the
  Rust rule removes noise. Only `calibration/` and `calibration-holdout/`
  show that.
- The TypeScript confirmation is stronger. The rule raises precision from
  39.5% to 91.4% on 2,230 pairs.
- Each set has one repository for each language here, so a single codebase
  decides each row.
- The labels are model judgments, not human labels. The reviewers of shards
  1 and 3 labeled one family at a time. The reviewer of shard 3 wrote
  rationales from fixed templates, so its rationales give no reasoning for
  each pair.
- The reviewer of shard 0 labeled 89 pairs of openpage-api routes as copy.
  The reviewer noted that required-shape is also defensible for them. The
  TypeScript rule keeps 6 openpage-api pairs, all labeled copy. If those 6
  are non-copies, TypeScript precision is 122/140 = 87.1%.
- Recall is low: the rules keep 25% of the Rust copies and 15% of the
  TypeScript copies.
- Fixture data from documenso's `packages/app-tests` passed the test
  exclusion.
