#!/bin/sh
# The current-CLI reproductions behind docs/labeling-2026-09-21.md. Run from the repository root.
# Every case asserts its expected exit status and a line of the expected report, and the script
# exits non-zero on the first case that drifts.
set -u
KLIN="${KLIN_BIN:-$PWD/target/release/klin}"
FIXTURE="$PWD/benchmark/fixtures/reachability/base"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

fresh() {
  rm -rf "$ROOT/tree" && mkdir -p "$ROOT/tree" && cd "$ROOT/tree" && git init -q
}

expect() {
  name="$1"; status="$2"; pattern="$3"; shift 3
  out="$("$KLIN" "$@" 2>&1)"; got=$?
  if [ "$got" -ne "$status" ] || ! printf '%s\n' "$out" | grep -Eq "$pattern"; then
    printf 'FAILED: %s\n  expected exit %s matching /%s/, got exit %s:\n%s\n' "$name" "$status" "$pattern" "$got" "$out"
    exit 1
  fi
  printf 'ok: %s (exit %s)\n' "$name" "$got"
}

fresh && git commit -q --allow-empty -m base && mkdir -p src tests && echo '{}' > klin.json
printf 'pub fn wrap(t: &str) -> Vec<String> { vec![t.to_string()] }\n' > src/lib.rs
printf 'use demo::wrap;\n#[test]\nfn keeps() {\n    let lines = wrap("ab");\n    assert_eq!(lines.last().unwrap(), "ab");\n}\n' > tests/render.rs
expect "escapes flags unwrap in a Rust integration test (#279)" 1 '^FAIL: 1 new escape site' escapes

fresh && cp -R "$FIXTURE/." . && git add -A && git commit -q -m base && git checkout -q -b work
sed -i.bak 's/add_command::run_add, //; /"add" => run_add(rest, store),/d' src/registry.rs
sed -i.bak '/pub mod add_command;/d' src/commands/mod.rs && rm -f src/registry.rs.bak src/commands/mod.rs.bak
expect "reachability fails a family file left unreferenced" 1 'add_command.rs:0  unreached' reachability

git checkout -q -- . && printf 'pub fn run_add2() {}\n' > src/commands/add2_command.rs
expect "reachability fails an orphan inside the derived family" 1 'add2_command.rs:0  unreached' reachability

rm src/commands/add2_command.rs && cp src/commands/add_command.rs src/commands/add.rs
expect "a copied implementation outside the family passes every gate (#48 scope)" 0 'gate\(s\), all passed' gate

printf 'fn helper_only_here() {}\n' > src/commands/add.rs
expect "dead-symbols fails a private orphan outside the family" 1 'add.rs:1  dead' dead-symbols

echo "every reproduction holds"
