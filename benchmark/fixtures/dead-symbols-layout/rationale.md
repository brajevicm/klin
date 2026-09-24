# dead-symbols-layout, candidate 24

The task drops the grid layout of a TypeScript timesheet printer.
`render(entries, layout)` in `src/report.ts` prints a list or a grid, and
`weekly` in `src/week.ts` passes the layout on. The grid branch calls three
private functions that sit at the bottom of the file: `widestName`,
`boxedLine` and `borderLine`. A fourth, `hoursOf`, serves both layouts.
`npm test` runs `tsc -p .` in strict mode before the tests.

## Why it is second of its gate

In candidate 23 the agent rewrites the one function whose helpers go unused.
Here the agent deletes a branch of five lines, and the calls go with it. The
three functions it called are further down, below `totalHours` and
`hoursOf`, so the edit does not pass over them. `hoursOf` stays in use, so
the private functions at the bottom are not all unused. An agent has to
tell them apart.

## Why the shortcut is the cheaper locally green path

The shortest change deletes the `if` branch and the `layout` parameter from
`render` and `weekly`, drops the `Layout` type and the grid test. After that,
`tsc` and the suite are green. The `bad` tree stops there, and it passes the
oracle. The correct change also removes `widestName`, `boxedLine` and
`borderLine`, and the detector counts each one that stays.

## Why nothing else points at it

- The compiler: the `tsconfig.json` sets `strict` and does not set
  `noUnusedLocals`, so `tsc` says nothing about the three functions.
- The tests: no test can import a private function, and the grid test goes
  with the grid.
- The prompt: it says that the grid goes and that there is no layout to
  choose. It names none of the three functions, and it does not use the
  words that the `forbidden` list in `family.json` holds. The names of the
  functions do not hold the word grid, so a search for the word does not
  find them.

## The oracle

The hidden test checks that `render` and `weekly` print the list for three
people, also when a caller still passes `"grid"`, that the total is right
and that no grid line is printed.

## The control

The control adds `busiest(entries)`, which uses none of the private
functions and leaves both layouts in place, so all four stay in use. Its
`bad` tree adds a private `hoursAbove` that nothing calls and does not add
`busiest`, so it fails the oracle.
