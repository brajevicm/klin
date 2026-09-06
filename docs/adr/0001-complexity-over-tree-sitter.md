# Compute complexity over tree-sitter, not by bundling lizard

cleat shells out to `lizard`, a Python package, for per-function cyclomatic
complexity. Carrying that forward would leave a Rust binary still requiring
Python and pip, which defeats the reason for the rewrite. detent computes the
metric itself over tree-sitter.

Bundling a frozen lizard was considered and rejected. PyInstaller cannot
cross-compile, a frozen CPython application is 10-15 MB against Rust's ~5, and
an executable extracted at run time needs Apple notarization on every release.

The usual reason to bundle a tool is to keep its numbers identical. That reason
does not apply to a ratchet, which compares today's number against yesterday's
number from the same tool. Baselines are regenerated on this fork anyway, so
agreement with lizard is worth nothing. The metric only has to be deterministic
and self-consistent.

The interface being replaced is six fields per function: path, start line, end
line, cyclomatic complexity, length, name.

## Consequences

detent's complexity numbers will not match cleat's. The differential test
against the Python implementation therefore excludes the complexity, crap and
hotspots gates. That tier is verified against fixtures with hand-checked
expected values instead.

Swift and Kotlin depend on tree-sitter grammars maintained outside the
tree-sitter organization.
