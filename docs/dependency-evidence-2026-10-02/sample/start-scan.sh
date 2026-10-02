#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: start-scan.sh CLONES}
deps=$here/../prototype/deps.py
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$here/start-undeclared.tsv"
while read -r name start; do
  rm -rf "$work/tree"
  mkdir -p "$work/tree"
  git -C "$clones/${name//\//__}" archive "$start" | tar -x -C "$work/tree"
  python3 "$deps" imports "$work/tree" | awk -F'\t' -v n="$name" '$5 != "declared" { print n "\t" $0 }' >> "$here/start-undeclared.tsv"
done < <(jq -r '.repositories[] | "\(.fullName) \(.start)"' \
  "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json")
awk -F'\t' '$5 == "-" && $6 != "unknown" { split($6, v, ","); for (i in v) { k = $1 "\t" $2 "\t" v[i]; if (!seen[k $3]++) n[k]++ } }
  END { for (k in n) print k "\t" n[k] }' "$here/start-undeclared.tsv" | sort > "$here/start-undeclared-summary.tsv"
cat "$here/start-undeclared-summary.tsv"
