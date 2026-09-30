# Contributing to klin

## Start with an issue

Small fixes can go straight to a pull request. For anything bigger, open an issue first, and we'll agree on the change before you build it.

## Using AI

AI help is welcome. klin is built with agents, too. Just keep a person in the loop:

- You open the pull request, and you've read every line of it.
- You say which agent helped.
- You answer the review.

Pull requests from agents working alone are closed.

## Before you open a pull request

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked
cargo build --release --locked && ./target/release/klin gate --strict
```

If a gate fails, fix the code it points to. Leave `klin.json`, the hooks and the `accepted` list alone. Only the maintainer changes them, in a reviewed commit.

Changing a check? Read `AGENTS.md` first.
