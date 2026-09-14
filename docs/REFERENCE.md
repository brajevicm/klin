# klin configuration reference

`klin reference` prints this page. `docs/REFERENCE.md` holds the printed copy, and a test fails when the two differ, so the reference cannot drift from the binary. Do not edit the copy by hand.

`klin.json` is optional. Automatic checks run from facts discovered in the tree. A section may pin human policy and leave the rest to derivation, and a run prints one `pinned:` or `derived:` line per value it used. A top-level key klin does not read is an error naming the key. Compact source sections also reject any field their table does not name. A gate is excluded by setting its section to `false`.

## Top-level keys

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `project` | a name for reports | no | pinned only | — | no name |
| `version` | the klin version this configuration was written for. A run under another version prints a NOTE naming both and continues | no | pinned only | — | no version |
| `build` | the commands a run builds with, each an entry of a `run` and an optional `root` | no | derived when absent | one entry per manifest, from the fixed table of ADR 0012 | — |
| `accepted` | the debt a person accepted, each entry a site and a reason. Only a person writes it | no | pinned only | — | nothing is accepted |
| `radius` | the change radius a turn may not pass, as `lines` and `directories` | no | derived when absent | the 90th percentile over the last 200 non-merge commits, and no section below 50 commits | — |
| `journal` | how the journal records a turn, as `prompt`, `false` to record no prompt excerpt | no | pinned only | — | the prompt excerpt is recorded |

## Sections

One key per gate, named for its section. `complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability` discover repository topology themselves; their objects contain only human policy. A section reads only the keys its own table names.

### `doc_size`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `file` | the document this entry judges, as a path under the tree root | yes | derived with the section | one entry per Markdown file at the tree root that the derivation commit holds | — |
| `ceiling` | the words the document may not pass | yes | derived with the section | the word count at the derivation commit, rounded up to the next 50 and never below 50 | — |

### `doc_citations`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `file` | the document this entry reads citations from | yes | derived with the section | one entry per Markdown file at the tree root | — |
| `roots` | the directories a citation may resolve under | no | derived with the section | the tree root | the tree root |
| `extensions` | the file extensions a citation may name | no | pinned only | — | `.py`, `.ts`, `.tsx`, `.js`, `.jsx`, `.swift`, `.rs`, `.go`, `.kt`, `.java`, `.rb`, `.sh`, `.md`, `.json`, `.yml`, `.yaml`, `.toml` |

### `lockfile`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `manifests` | the manifest files this check proves against their lockfiles | yes | derived when absent | one entry per manifest klin has a lockfile reader for: `Cargo.toml`, `package.json` and `go.mod` | — |
| `exclude` | globs on the basename and on the path from the tree root | no | pinned only | — | nothing is excluded |

### `escapes`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |
| `skip_rust_tests` | whether an inline Rust test module is left out | no | pinned only | — | `true` |

### `stubs`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |

### `inventory`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `name` | what a finding calls this entry | yes | derived with the section | the test root's path | — |
| `path` | the directory whose test sites this entry holds | yes | derived with the section | one entry per test root the survey found | — |
| `pattern` | a glob on the basename that limits the entry | no | pinned only | — | every file under `path` |

### `complexity`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `cc` | the cyclomatic complexity a function may not pass | no | derived when absent | the 95th percentile of `cc` over every supported function selected by the compact scope recorded at the derivation commit, rounded up to the next whole number, with a floor of 5, and the floor itself below 50 functions | — |
| `lines` | the body length a function may not pass | no | derived when absent | the 95th percentile of `lines`, by the same rule as `cc`, with a floor of 25 | — |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |

### `dead_symbols`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |
| `ignore` | name globs for declarations the check leaves out | no | pinned only | — | Rust `main`, test functions and declarations marked externally visible |

### `reachability`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |

### `conventions`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `text` | the literal text no line may hold, matched as written. Each convention states one of `text`, `code` and `files` | no | pinned only | — | — |
| `code` | a code pattern no source may hold, with `$NAME` for one piece of code and `$$$ARGS` for a list. A fragment, such as a match arm or a type, is read everywhere the language holds one | no | pinned only | — | — |
| `files` | a glob over repository-relative paths no file may sit at, where `*` stays inside one directory and `**/` crosses any number | no | pinned only | — | — |
| `remedy` | the exact action to take instead, printed with every failure | yes | pinned only | — | — |
| `in` | a repository-relative path, or a list of them, the section applies to, with everything below each | no | pinned only | — | the whole repository |
| `except` | a repository-relative path, or a list of them, taken out of `in`, with everything below each | no | pinned only | — | nothing is taken out |
| `language` | the language a `code` pattern is written in | no | pinned only | — | the one language the source in scope is written in |

### `sarif`

| Key | Holds | Required | Source | Derivation rule | Default |
| --- | --- | --- | --- | --- | --- |
| `name` | the gate's own name, which `--gate` takes | yes | pinned only | — | — |
| `report` | the SARIF file this gate reads | yes | pinned only | — | — |
| `run` | the command that writes the report before the gate reads it | no | pinned only | — | klin reads the report as it finds it and refuses one that predates the change |
| `differential` | whether only a finding on a line the window changed is judged | no | pinned only | — | `false` |

## Built-in language coverage

These tables report the source extensions each check discovers automatically. They are capabilities of the binary, not selectors accepted in `klin.json`.

### `escapes`

| Name | Extensions |
| --- | --- |
| `go` | `.go` |
| `java` | `.java` |
| `javascript` | `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs` |
| `kotlin` | `.kt`, `.kts` |
| `python` | `.py` |
| `ruby` | `.rb` |
| `rust` | `.rs` |
| `shell` | `.sh`, `.bash`, `.zsh` |
| `swift` | `.swift` |
| `typescript` | `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs` |

### `stubs`

| Name | Extensions |
| --- | --- |
| `go` | `.go` |
| `javascript` | `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs` |
| `python` | `.py` |
| `rust` | `.rs` |
| `typescript` | `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs` |

### `complexity`

| Name | Extensions |
| --- | --- |
| `go` | `.go` |
| `java` | `.java` |
| `javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` |
| `kotlin` | `.kt`, `.kts` |
| `python` | `.py` |
| `ruby` | `.rb` |
| `rust` | `.rs` |
| `swift` | `.swift` |
| `tsx` | `.tsx` |
| `typescript` | `.ts`, `.mts`, `.cts`, `.tsx` |

### `dead_symbols`

| Name | Extensions |
| --- | --- |
| `rust` | `.rs` |
| `typescript` | `.ts`, `.mts`, `.cts`, `.tsx` |

### `reachability`

| Name | Extensions |
| --- | --- |
| `rust` | `.rs` |
| `typescript` | `.ts`, `.mts`, `.cts`, `.tsx` |

### `conventions`

| Name | Extensions |
| --- | --- |
| `rust` | `.rs` |
| `typescript` | `.ts`, `.mts`, `.cts`, `.tsx` |

## Scope and discovery

Every source check discovers supported files from one repository walk, skips the fixed directory list `.git`, `node_modules`, `vendor`, `build`, `.build`, `dist`, `target`, `__pycache__`, `.venv`, `venv`, `DerivedData`, `Pods`, `coverage`, `.next`, `out`, `fixtures`, and drops files git ignores. `in` narrows a check to a repository-relative path (or non-empty list) and `except` takes paths back out. Each path names itself and everything below it; neither key accepts globs.

`complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability` reject the retired `roots`, `languages`, `patterns`, `skip_dirs`, `exclude`, `exclude_except` and `ceilings` topology keys with a migration error. A file measured under the base scope and omitted by today's scope is a NOTE in the hook and exit 2 under `--strict`.

Other sections retain the selection keys their own tables name.

## Ceilings

A pinned ceiling is either a whole number or an object of dated steps:

```json
"complexity": {
  "cc": 12,
  "lines": { "2026-09-08": 90, "2027-01-01": 70, "2027-07-01": 60 }
}
```

The run uses the lowest step whose date is on or before today, in UTC. A schedule with no step yet due is an error. A run that uses a schedule prints the date it used beside the ceiling. A derived ceiling is not monotone: it falls when simple functions arrive and rises when simple functions leave, so a person who wants a ceiling that cannot loosen pins one.
