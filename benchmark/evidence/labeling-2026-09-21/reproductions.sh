#!/bin/sh
# The current-CLI reproductions behind docs/labeling-2026-09-21.md. Run from the repository root.
set -u
KLIN="${KLIN_BIN:-$PWD/target/debug/klin}"
FIXTURE="$PWD/benchmark/fixtures/reachability/base"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

fresh() {
  rm -rf "$ROOT/tree" && mkdir -p "$ROOT/tree" && cd "$ROOT/tree" && git init -q
}

echo "== escapes: unwrap in a Rust integration test (expected: FAIL, the #279 gap)"
fresh && git commit -q --allow-empty -m base && mkdir -p src tests && echo '{}' > klin.json
printf 'pub fn wrap(t: &str) -> Vec<String> { vec![t.to_string()] }\n' > src/lib.rs
printf 'use demo::wrap;\n#[test]\nfn keeps() {\n    let lines = wrap("ab");\n    assert_eq!(lines.last().unwrap(), "ab");\n}\n' > tests/render.rs
"$KLIN" escapes; echo "exit=$?"

echo "== reachability: family file left unreferenced (expected: FAIL, valid signal)"
fresh && cp -R "$FIXTURE/." . && git add -A && git commit -q -m base && git checkout -q -b work
sed -i.bak 's/add_command::run_add, //; /"add" => run_add(rest, store),/d' src/registry.rs
sed -i.bak '/pub mod add_command;/d' src/commands/mod.rs && rm -f src/registry.rs.bak src/commands/mod.rs.bak
"$KLIN" reachability; echo "exit=$?"

echo "== reachability: orphan inside the derived family (expected: FAIL)"
git checkout -q -- . && printf 'pub fn run_add2() {}\n' > src/commands/add2_command.rs
"$KLIN" reachability; echo "exit=$?"

echo "== orphan outside the family with only pub items (expected: every gate OK, known scope boundary)"
rm src/commands/add2_command.rs && printf 'pub fn run_add() {}\n' > src/commands/add.rs
"$KLIN" gate | grep -E '^\s*(ok|FAIL|klin)'; echo "exit=$?"

echo "== orphan outside the family with a private item (expected: dead-symbols FAIL)"
printf 'fn helper_only_here() {}\n' > src/commands/add.rs
"$KLIN" dead-symbols; echo "exit=$?"
