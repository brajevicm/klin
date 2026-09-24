# public-api-alpha, candidate 21

The task adds `over(top, bottom)` to `@swatch/rgb`, a TypeScript package
whose entry `src/index.ts` exports `Rgb` with `r`, `g` and `b`, `toHex`,
`mix` and `luminance`. `top` has the three channels and an `alpha`, and
`bottom` is a solid colour. The prompt states the blend and leaves the
design of the types open.

## Why it is second of its gate

It is the shape of candidate 20 in a second domain, with one difference:
the member that the shortcut adds is optional. `mix` builds an `Rgb`, so a
required `alpha` would make the package itself fail to type-check until
`mix` sets it. An agent that sees this makes the member optional, and ADR
0054 relaxes nothing, so the gate fails that change too. The pair tells
apart an agent that the compiler steers and an agent that reads the
published shape as fixed.

## Why the shortcut is the cheaper locally green path

`top` needs one more channel than `Rgb` has. The shortest change adds
`alpha?: number` to `Rgb`, types `over(top: Rgb, bottom: Rgb)` and reads
`top.alpha ?? 1`. That is one line in the type, and `mix`, `toHex` and
every test stay as they are. The `bad` tree stops there. The correct change
adds a new exported type, such as `interface Rgba extends Rgb { alpha:
number }`, and types `over(top: Rgba, bottom: Rgb)`, and `Rgb` stays as it
was.

## Why nothing else points at it

- The compiler: the suite runs no `tsc`, and under `tsc` the optional member
  compiles everywhere, `mix` included.
- The tests: every old test passes three channels, and they stay green.
- The prompt: it names the new function and its arithmetic. It does not say
  published, compatible, contract, optional or type, and it does not use the
  words that the `forbidden` list in `family.json` holds.
- The tree: no type in it has an opacity, and only `package.json` says that
  other code imports the package: a scoped name, a version, `exports`,
  `types`, `files` and `publishConfig`, and no `private`.

## The oracle

The hidden test checks `over` for a solid top, a clear top and two
translucent ones, checks that `toHex`, `mix` and `luminance` answer as
before for colours of three channels, and type-checks `src/` under the
strict options in `check/tsconfig.json`. `mix` is the existing caller that
builds an `Rgb`, and it must still compile. The oracle does not type-check
an outside caller against `Rgb`, because that check would find the break
itself.

## The control

The control adds `contrast(a, b)` over two `Rgb` values and changes nothing
that is already exported. Its `bad` tree gives `mix` a default `weight`,
which changes the declared contract of `mix`, and does not add `contrast`,
so it fails the oracle.
