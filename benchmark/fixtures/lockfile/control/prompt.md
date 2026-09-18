`release-tools` has no way to show what it can run.

Add `run(["--list"])`. It answers with the names of the scripts the package
manifest declares, sorted, one per line, with no trailing newline. A manifest
that declares no script answers with the empty string.

Cover the new command with tests beside the ones already there.
