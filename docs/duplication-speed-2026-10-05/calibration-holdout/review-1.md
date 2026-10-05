# Review 1: blind labels for blind-1.json

These are model judgments (Claude, blind source review), not human labels.

## Totals

The shard lists 638 pair entries across 383 families, but only 608 distinct pair ids: 30 entries repeat an id already listed in another family (mostly utf8.rs self-matches). labels.json has one entry per distinct id. No repeated id got different labels in different families.

| label | pairs |
|---|---|
| copy | 230 |
| boilerplate | 204 |
| required-shape | 150 |
| distinct | 13 |
| generated | 11 |
| mixed | 0 |

## Generated evidence

All 11 `generated` pairs are actual `packages/component-library/src/icons/v{0,1}/*.tsx` icon components. The evidence is in `packages/component-library/package.json`: the `generate:icons` script runs `svgr --template template.ts ... -d . .` over `src/icons`. Nothing else was labelled generated. Shape alone was not used as evidence.

## Distinct

These are all self-overlapping matches inside one constant table: the 256-entry `UTF8_CHAR_WIDTH` in `yazi-shim/src/utf8/utf8.rs`, and two overlapping ranges in `yazi-shared/src/translit/table.rs`. The rows repeat, but the two locations are not two independent copies.

## Hard cases and uncertainty

- **gitui popup `Component` impls.** Pairs whose matched text is mostly `is_visible`/`hide`/`show` or the input-popup `event` prelude are labelled required-shape. Bodies with real logic shared between two popups (inspect/compare commit event handling, pull/fetch credential handling, `move_selection`, where one site carries a "dedup" TODO) are labelled copy.
- **yazi `Progress` impls in `file/progress.rs`.** These are trait impls, but the running/cooked logic is identical across types, so I labelled them copy. Smaller per-type trait bodies are required-shape.
- **actual report pages.** Page/header JSX, recharts tooltip container shells and import preludes are boilerplate. The data-fetch effects, `onSaveWidget`, month clamping and range derivation are copy.
- **Platform variants** (`index.electron.ts` vs `index.ts`/`index.api.ts`, macOS vs freedesktop trash, windows vs macos locale). Identical kernels are labelled copy, following the rubric.
- **Vendored shadcn UI** (dropdown-menu, button) duplicated between linkwarden web and extension: boilerplate. A reviewer could argue copy.
- **Constant lists** (web/mobile font sizes and line heights, cloud config, title-case regex across packages): copy, since one shared module could own them. Dashboard default widget data and formula catalog entries: boilerplate.
- **Borderline calls** that could go either way: the AgreeDrag/AgreeDrop parse arms (boilerplate), the `on!` dispatch macro in yazi `executor.rs` (boilerplate, 45 pairs), the mobile tab layout search header (copy), and the initialise-modal submit handlers (copy).
- **Truncated snippets.** Most families were judged from the matched lines plus surrounding context, without opening every full file.
