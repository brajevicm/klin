# Review 0: blind labels for blind-0.json

These are model judgments made by a blind reviewer agent. No human produced them. I read the code in the snippets and, where I needed context, the pinned checkouts. I did not look at token lengths, thresholds, `out/`, mapping files, or other reviews.

## Totals

The shard lists 638 pair entries across 382 families. Those entries contain 605 unique pair ids, so `labels.json` has 605 keys. Eight ids show up in more than one family; each of them got the same label every time.

| label | unique pairs | pair entries |
|---|---|---|
| boilerplate | 281 | 281 |
| copy | 247 | 247 |
| required-shape | 47 | 47 |
| generated | 16 | 16 |
| distinct | 14 | 47 |
| mixed | 0 | 0 |

## Generated evidence

- actual icon components (`packages/component-library/src/icons/**`): `packages/component-library/package.json` defines `"generate:icons"`, which regenerates `src/icons/*/*.tsx` with `svgr --template template.ts`. Every icon pair is labeled generated.
- linkwarden `dropdown-menu.tsx` in apps/web and `DropDownMenu.tsx` in apps/extension (families 258 and 266): both apps contain a shadcn `components.json` with the `ui` alias, and these files are the shadcn CLI scaffold. This is weaker evidence than the svgr script. shadcn components are written once by the CLI and then belong to the project (their class strings already differ), so someone else could reasonably call these copy or boilerplate.

## Hard cases and uncertainty

- **utf8.rs and translit table self-matches → distinct.** Many families match rows of the `UTF8_CHAR_WIDTH` literal in yazi-shim, or the transliteration table, against overlapping or neighbouring rows of the same literal. Some pairs are the same span on both sides. None of them are two independent copies.
- **gitui Component impls.** I labeled `is_visible/hide/show` tails and plain `commands()` impls required-shape, and key-dispatch `event()` preludes and popup `draw` scaffolds boilerplate. When the shared body carries real behavior (push/pull/fetch credential delegation, branch-name warnings, the reset/checkout-option text), I labeled it copy. The boundary between those groups is a judgment call.
- **yazi `Data` deserializers (`de.rs` / `de_owned.rs`).** These are trait methods, but the owned and borrowed deserializers repeat identical bodies, so I labeled them copy. Serializer compound-trait impls in yazi-sftp `ser.rs` are forwarding stubs, so they are required-shape.
- **Duplicated type declarations across packages** (actual news feed types, GoCardless transaction types): required-shape, read as one schema contract declared on both sides. A reviewer who sees them as shareable types could pick copy.
- **Report pages in actual.** The JSX scaffolding (headers, tooltip containers, import preludes) is boilerplate. The logic repeated across reports (onSaveWidget, month-range computation, earliest/latest transaction fetch) is copy.
- **Platform splits** in yazi (trash backends, locale modules, terminal setup) are labeled copy, following the rule that a platform difference does not excuse an identical kernel.
- The playwright page-model pair (family 94) sits under `e2e/` and survived the upstream test exclusion. I labeled it boilerplate.
- I used no `mixed` labels. Within each family the pairs had equivalent contexts.
