# Compute complexity over tree-sitter, not by bundling lizard

klin computes per-function cyclomatic complexity over tree-sitter. Shelling
out to `lizard`, a Python package, would require Python and pip beside the
Rust binary.

Bundling a frozen lizard was considered and rejected. PyInstaller cannot
cross-compile, a frozen CPython application is 10-15 MB against Rust's ~5, and
an executable extracted at run time needs Apple notarization on every release.

The usual reason to bundle a tool is to keep its numbers identical. That reason
does not apply to a ratchet, which compares today's number against yesterday's
number from the same tool. The metric must be deterministic and
self-consistent; agreement with another tool is not a requirement.

The measurement carries six fields per function: path, start line, end
line, cyclomatic complexity, length, name.

## Consequences

Complexity measurements are verified against fixtures with hand-checked
expected values through klin's command line.

Swift and Kotlin depend on tree-sitter grammars maintained outside the
tree-sitter organization.
