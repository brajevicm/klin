# A named re-export is a `reachability` reference

> Amends spec 8.4. The name-only rule of ADR 0035, the family derivation and
> `dead-symbols` stand. What changes is what counts as a reference to a
> member's declaration.

`reachability` reads a member as reached when another file holds a reference
with the name of one of its eligible declarations. The structural index held
no reference for a TypeScript re-export. In the replay of #343
(`docs/false-alarms-2026-09-29.md`), R056 labeled `isTypenameField.ts` as not
appropriate: no file called it at head, but
`src/utilities/internal/index.ts:65` re-exported it from a published entry
point, so it is public surface. An agent drafted the label and agents reviewed
it.

## The decision

**A named re-export is a reference in `reachability`.**
`export { x } from "./m"` and `export { x as y } from "./m"` count as a
reference to `x` from the file that holds them, under the name-only rule.
Ambiguity still reads as reached: the re-export names every declaration of
`x`. A re-export also proves a member for the family derivation, as any other
reference does.

A re-export names a declaration by the name a consumer addresses it by. For
`export default function Profile` that name is `default`, so
`export { default } from "./m"` and `export { default as Profile } from "./m"`
both name it, and `export { Profile } from "./m"` does not. A re-export proves
a member only where one declaration under the index answers to that name, so
a `default` re-export proves nothing while several files export a default.

The rule reads the export facts klin already extracts. It resolves no module,
parses nothing a second time and walks no new tree. The check builds one map
from each re-exported name to the files that re-export it, over the index it
already holds.

**`export *` stays unresolved.** A star re-export names no declaration, so
naming one would take module resolution. It stays a known limit of spec 8.4.

**`dead-symbols` does not change.** It judges private declarations, which no
re-export can name. The index that both checks read gets no new reference, so
a re-export in one file cannot make a private declaration of the same name in
another file look referenced.

## Rejected

- **Add re-exports to the shared index as references.** `dead-symbols` would
  then read `export { helper } from "./other"` as keeping every private
  `helper` alive.
- **Resolve the specifier and reach only the file it names.** That takes the
  module graph of `layering`, and the name-only rule already accepts that
  ambiguity reads as reached.
- **Resolve `export *` through the target's exports.** It takes module
  resolution too, and no labeled row needs it.
