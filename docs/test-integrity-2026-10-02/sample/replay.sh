#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: replay.sh CLONES PILOT_CLONES}
pilot=${2:?usage: replay.sh CLONES PILOT_CLONES}
asserts_target=${ASSERTS_TARGET:-$(mktemp -d)}
CARGO_TARGET_DIR=$asserts_target cargo build --quiet --release --manifest-path "$here/../prototype/Cargo.toml"
asserts=$asserts_target/release/asserts

export_tree() {
  rm -rf "$3"
  mkdir -p "$3"
  git -C "$1" archive "$2" | tar -x -C "$3"
}

changes() {
  jq -r '.repositories[] | .fullName as $name | .changes[] | "ordinary \($name) \(.base) \(.head)"' \
    "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json"
  jq -r '.repositories[] | .fullName as $name | .changes[] | "tests \($name) \(.base) \(.head)"' "$here/selection-tests.json"
  jq -r '(.agent[] | "agent \(.language) \(.fullName) \(.base) \(.head)"), (.human[] | "human \(.language) \(.fullName) \(.base) \(.head)")' \
    "$repo/docs/phenotype-pilot-2026-10-02/selection.json" | awk '$2 == "Rust" || $2 == "TypeScript" { print $1, $3, $4, $5 }'
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$here/changes.tsv"
: > "$here/sites.tsv"
while read -r set name base head; do
  dir=$clones/${name//\//__}
  [ "$set" = ordinary ] || [ "$set" = tests ] || dir=$pilot/${name//\//__}
  head=$(git -C "$dir" rev-parse "$head")
  files=()
  while read -r file; do files+=("$file"); done < <(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head" | grep -E '\.(rs|ts|mts|cts|tsx)$' || true)
  counts="0	0	0"
  if [ ${#files[@]} -gt 0 ]; then
    export_tree "$dir" "$base" "$work/before"
    export_tree "$dir" "$head" "$work/after"
    "$asserts" new "$work/before" "$work/after" "${files[@]}" 2> "$work/counts" | sed "s|^|$set\t$name\t${head:0:10}\t|" >> "$here/sites.tsv"
    counts=$(cut -f2- "$work/counts")
  fi
  langs=$(printf '%s\n' "${files[@]:-}" | awk '/\.rs$/ { r = 1 } /\.(ts|mts|cts|tsx)$/ { t = 1 } END { print (r ? "rust" : "") (r && t ? "," : "") (t ? "ts" : "") }')
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$set" "$name" "${head:0:10}" "${#files[@]}" "${langs:--}" "$counts" >> "$here/changes.tsv"
done < <(changes)
wc -l "$here/changes.tsv" "$here/sites.tsv"
