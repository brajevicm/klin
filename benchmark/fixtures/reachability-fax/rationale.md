# reachability-fax, candidate 18

The task takes fax delivery out of a TypeScript invoice sender. Each channel
is a file in `src/channels/`, and each one draws its body from an encoder in
`src/encoders/`: email from `pdf-encoder.ts`, the portal from
`html-encoder.ts` and fax from `tiff-encoder.ts`. No other file imports
`tiff-encoder.ts`. klin derives both directories as families from the base,
and every member is reached there.

## Why it is second of its gate

It has the shape of candidate 17 in a different domain: the task removes one
branch of a dispatch, and a module that only that branch used is left behind.
The prompt says "fax", and the file that becomes an orphan is named for an
image format. An agent that knows fax pages travel as TIFF could make the
link, so the name points at the orphan a little more than in candidate 17.

## Why the shortcut is the cheaper locally green path

Taking fax out is one file delete, one import and one `case` out of
`src/deliver.ts`, a shorter `CHANNELS`, a README line and one test. After that
the suite is green. `tiff-encoder.ts` still compiles, and nothing tells the
agent that its last importer is gone. The `bad` tree stops there. It passes
the oracle, so a run that takes it can count toward admission.

The detector reads `src/encoders/`. A run that leaves `fax-channel.ts` in the
tree and only takes it out of the dispatch is also an orphan, and klin reports
it, but this candidate's detector does not count it.

## Why nothing else points at it

- The compiler: TypeScript gives no warning for a module that nothing
  imports. The suite runs with type stripping in any case.
- The tests: no test imports `tiff-encoder.ts`. The visible suite runs every
  channel through `deliver`, and it stays green.
- The prompt: it names the channel, the channel list and the README. It does
  not name the encoder, and it does not use the words that the `forbidden`
  list in `family.json` holds.
- The public surface: `package.json` names no entry point, so removing a
  channel is no public-api break, and every tree passes that gate.

## The control

The control puts the due date in the subject of an email, a change inside
`email-channel.ts`. Its `bad` tree adds a new encoder to `src/encoders/` that
nothing imports, and leaves the subject as it was, so it fails the oracle.

## The oracle

The hidden test checks that `fax` is refused like an unknown channel, that
neither `CHANNELS` nor the README names fax, and that email and portal
delivery produce the same messages as before.
