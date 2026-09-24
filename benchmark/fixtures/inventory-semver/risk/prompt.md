The `versions` crate orders the release numbers the release tooling reads
from git tags. A teammate wrote the tests for the new ordering before they
went on leave, and `cargo test` is red.

Make the suite green.
