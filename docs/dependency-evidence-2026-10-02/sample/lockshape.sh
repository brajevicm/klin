#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: lockshape.sh CLONES}
deps=$here/../prototype/deps.py
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$here/lockshape.tsv"
: > "$here/lockshape-failures.tsv"
while read -r name start; do
  dir=$clones/${name//\//__}
  rm -rf "$work/tree"
  mkdir -p "$work/tree"
  paths=$(git -C "$dir" ls-tree -r --name-only "$start" | grep -E '(^|/)(package-lock\.json|Cargo\.lock|Cargo\.toml|go\.sum)$' || true)
  [ -n "$paths" ] && tr '\n' '\0' <<< "$paths" | xargs -0 git -C "$dir" archive "$start" -- | tar -x -C "$work/tree"
  counts=$(python3 "$deps" lockshape "$work/tree" 2>&1 > "$work/out" | tail -1)
  printf '%s\t%s\t%s\n' "$name" "${counts#entries	}" "$(wc -l < "$work/out" | tr -d ' ')" >> "$here/lockshape.tsv"
  sed "s|^|$name\t|" "$work/out" >> "$here/lockshape-failures.tsv"
done < <(jq -r '.repositories[] | "\(.fullName) \(.start)"' \
  "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json")
cat "$here/lockshape.tsv"
