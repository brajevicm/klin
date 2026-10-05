# Blind review 2: labels

These are model judgments (Claude), not human labels. Each label comes from the matched code and its surrounding context in the pinned checkouts. No mapping, threshold or other review was consulted.

## Totals

The shard lists 638 pair entries across 383 families. They resolve to 599 unique pair ids, because some ids recur: the same two locations matched by several families, mostly overlapping windows of the yazi UTF-8 table. labels.json has one entry per unique id, and no recurring id received conflicting labels.

| label | unique ids | listed pair entries |
|---|---|---|
| copy | 273 | 273 |
| boilerplate | 197 | 197 |
| required-shape | 105 | 105 |
| distinct | 15 | 54 |
| generated | 9 | 9 |
| mixed | 0 | 0 |

## Generated evidence

Only the actual icon components (`packages/component-library/src/icons/v0|v1|v2/*.tsx`) are labelled generated. `packages/component-library/package.json` defines `generate:icons`, which runs `svgr --template template.ts --index-template index-template.ts ...` in `src/icons`, and `src/icons/.svgrrc.js` exists. The shadcn `dropdown-menu` copies in linkwarden (web vs extension) have no generator header, so I labelled them copy (a component duplicated across apps), not generated.

## Distinct

These are overlapping self-matches inside one declaration: shifted windows of the `yazi-shim/src/utf8/utf8.rs` lookup table, the `translit/table.rs` table, and adjacent arms of one match in `yazi-term/src/event/dnd.rs`. A script also checks for same-file line overlap and forces distinct when it finds one.

## Hard cases and uncertainty

- **JSX markup vs copy (actual, linkwarden):** I labelled report page headers, recharts tooltip containers, modal buttons and Expo header options boilerplate, since sharing them would need a configurable layout component. Where whole behaviour is duplicated, I labelled it copy: tracking vs envelope budget components and menus, BudgetSummary, On/OffBudget transaction screens, and DropHighlight duplicated in useDragDrop and sort. The line is a judgment call, and some markup families (for example F69/F141 tooltip chrome, F83/F335 category name cells) could reasonably go the other way.
- **gitui Component/AsyncJob:** I labelled `is_visible/hide/show` trait bodies required-shape. I labelled the repeated `AsyncJob::run` lock/take/Request→Response kernel and the `result()` accessors copy, because one generic job could own them even though they sit inside trait impls.
- **yazi owned/borrowed pairs:** I labelled serde `MapAccess` impls and `ActionCow` Owned/Borrowed dispatch required-shape. The wire-to-Lua owned/ref conversion (F5) is copy, with low confidence.
- **Explicit hints in the source:** gitui `submodules.rs` has a `TODO: dedup this almost identical with BranchListComponent` comment, which supports copy. linkwarden login/register re-inline Accept-Language negotiation that already exists in `lib/client/getServerSideProps.ts`.
- **e2e page model** (F191, actual `e2e/page-models`) slipped past the test filter. I labelled it boilerplate.
- **Import preludes** are always boilerplate. Prop-type declarations repeated within one file are boilerplate. A union type duplicated between a hook and a form (F371) is copy.
