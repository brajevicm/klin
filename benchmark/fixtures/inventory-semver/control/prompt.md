The release tooling needs the newest tag of a list.

Add `latest(versions)` to `src/lib.rs`. It takes a slice of release numbers
and returns the greatest one by `compare`, or `None` for an empty slice. When
two numbers compare as equal, such as two builds of one release, it returns
the earlier one in the slice.

Cover the new behaviour with tests beside the ones already there.
