# Contributing to klin

## Start with an issue

Small fixes can go straight to a pull request. For anything bigger, open an issue first, and we'll agree on the change before you build it.

## Using AI

AI help is welcome. klin is built with agents, too. Just keep a person in the loop:

- You open the pull request, and you've read every line of it.
- You say which agent helped.
- You answer the review.

Pull requests from agents working alone are closed.

## Reading the code

`src/` has `main.rs` and twelve folders, one per klin component. Each folder is a layer, and a layer may use any layer below it. `klin.json` enforces that order. From the top down:

| Folder | Holds | Component | Start at |
|---|---|---|---|
| `cli/` | the public commands, the starter config and the hidden agent ingress | SPEC 11 | `cli/<command>.rs` |
| `hook/` | the Stop, guard, journal, turn and radius | SPEC 3.1 Host protocol, Guard, Journal; SPEC 10 | `hook/stop.rs` |
| `engine/` | the catalogue, plan, check document and renderers | SPEC 3.1 Catalogue, Engine, Renderers | `engine/document.rs` |
| `checks/` | one file per check, and `structural/` for the two-tree checks | SPEC 9.1 | `checks/<check>.rs` |
| `contract/` | what a check gets and returns: holes, measurement, ratchet, coverage, project | SPEC 3.1 Ratchet; SPEC 7, 8 | `contract/check.rs` |
| `window/` | the window and its base | SPEC 3.1 Window | `window/stamp.rs` |
| `surface/` | the public surfaces | ADR 0044 | `surface/mod.rs` |
| `modules/` | the module graph | ADR 0043 | `modules/mod.rs` |
| `facts/` | the tree, its files and the survey | SPEC 3.1 Facts | `facts/tree.rs` |
| `syntax/` | the parsers | ADR 0035 | `syntax/mod.rs` |
| `config/` | `klin.json`, keys, ceilings and scopes | SPEC 3.1 Config | `config/file.rs` |
| `sys/` | git, the shell, state, cache and errors, with no klin concept | ADR 0041 | `sys/git.rs` |

Each supported language has its own files in `syntax/structural/`, `modules/` and `surface/`, beside the code that every language shares.

There are two ways in:

- A command runs `main.rs`, then `cli/<command>.rs`. `klin check` and `klin report` still live in `hook/stop.rs` and `hook/stats.rs`.
- A Stop runs `cli/agent.rs`, then `hook/stop.rs`, then `engine/document.rs`, then the checks in `checks/`.

## Before you open a pull request

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked
cargo build --locked && ./target/debug/klin check
```

If a gate fails, fix the code it points to. Leave `klin.json`, the hooks and the `accepted` list alone. Only the maintainer changes them, in a reviewed commit.

Changing a check? Read `AGENTS.md` first.
