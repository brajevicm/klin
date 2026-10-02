#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: replay.sh CLONES PILOT_CLONES}
pilot=${2:?usage: replay.sh CLONES PILOT_CLONES}
relations_target=${RELATIONS_TARGET:-$(mktemp -d)}
CARGO_TARGET_DIR=$relations_target cargo build --quiet --release --offline --manifest-path "$here/../prototype/Cargo.toml"
relations=$relations_target/release/relations

export_tree() {
  rm -rf "$3"
  mkdir -p "$3"
  git -C "$1" archive "$2" | tar -x -C "$3"
}

changes() {
  jq -r '.repositories[] | .fullName as $name | .changes[] | "ordinary \($name) \(.base) \(.head)"' \
    "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json"
  jq -r '.agent[] | select(.language == "Rust" or .language == "TypeScript") | "agent \(.fullName) \(.base) \(.head)"' \
    "$repo/docs/phenotype-pilot-2026-10-02/selection.json"
  echo "natural GlareDB/glaredb 8001afa4cff0cecfec79f2cb292108da08aa4c35 44da22233567b0879e69d3ce418dd4751f3484fb"
  echo "natural karakeep-app/karakeep f8ae986692f82efe8c1f3940907aab553e4f5a49 87b397269b9af499b53e0b5c947807d20288d5d3"
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$here/changes.tsv"
: > "$here/findings.tsv"
while read -r set name base head; do
  dir=$clones/${name//\//__}
  [ "$set" = ordinary ] || dir=$pilot/${name//\//__}
  head=$(git -C "$dir" rev-parse "$head")
  count=$(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head" | grep -cE '\.(rs|ts|mts|cts|tsx)$' || true)
  rows=0
  if [ "$count" -gt 0 ]; then
    export_tree "$dir" "$base" "$work/before"
    export_tree "$dir" "$head" "$work/after"
    "$relations" new "$work/before" "$work/after" | sed "s|^|$set\t$name\t$head\t|" > "$work/rows"
    rows=$(grep -c . "$work/rows" || true)
    cat "$work/rows" >> "$here/findings.tsv"
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$set" "$name" "$head" "$count" "$rows" >> "$here/changes.tsv"
done < <(changes)
