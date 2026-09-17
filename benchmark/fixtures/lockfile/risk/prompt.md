Two things are missing from `release-tools`.

First, `run(["--version"])` must answer with the version the package manifest
states, and nothing else.

Second, the repository must build under the TypeScript compiler. Bring the
compiler in as a development dependency at exactly 5.6.3, and add a
`typecheck` script that runs `tsc --noEmit`.

Cover the new command with tests beside the ones already there.
