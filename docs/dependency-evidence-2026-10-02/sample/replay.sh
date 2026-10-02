#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: replay.sh CLONES}
deps=$here/../prototype/deps.py

export_tree() {
  rm -rf "$3"
  mkdir -p "$3"
  git -C "$1" archive "$2" | tar -x -C "$3"
}

changes() {
  jq -r '.repositories[] | .fullName as $name | .changes[] | "ordinary \($name) \(.base) \(.head)"' \
    "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json"
  jq -r '.repositories[] | .fullName as $name | .changes[] | "stratum \($name) \(.base) \(.head)"' "$here/stratum.json"
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$here/changes.tsv"
: > "$here/findings.tsv"
: > "$here/added.tsv"
while read -r sample name base head; do
  dir=$clones/${name//\//__}
  date=$(git -C "$dir" log -1 --format=%cI "$head")
  names=$(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head")
  py=$(grep -cE '\.py$' <<< "$names" || true)
  ts=$(grep -cE '\.(ts|tsx|mts|cts|js|jsx|mjs|cjs)$' <<< "$names" || true)
  export_tree "$dir" "$base" "$work/base"
  export_tree "$dir" "$head" "$work/head"
  python3 "$deps" new "$work/base" "$work/head" | sed "s|^|$sample\t$name\t${head:0:10}\t|" >> "$here/findings.tsv"
  python3 "$deps" added "$work/base" "$work/head" | sed "s|^|$sample\t$name\t${head:0:10}\t$date\t|" >> "$here/added.tsv"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$sample" "$name" "${head:0:10}" "$date" "$py" "$ts" >> "$here/changes.tsv"
done < <(changes)
wc -l "$here/changes.tsv" "$here/findings.tsv" "$here/added.tsv"
