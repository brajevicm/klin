#!/usr/bin/env bash
# holdout.sh CLONES: write selection-holdout.json. For each sample repository,
# the ten commits before the oldest base of its sample changes, on the
# first-parent walk, each judged against its first parent.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: holdout.sh CLONES}
jq -r '.repositories[] | select(.language != "Rust") | "\(.language)\t\(.fullName)\t\([.changes[].base] | join(" "))"' \
  "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json" |
while IFS=$'\t' read -r lang name bases; do
  dir=$clones/${name//\//__}
  oldest=$(for b in $bases; do printf '%s %s\n' "$(git -C "$dir" log -1 --format=%ct "$b")" "$b"; done | sort -n | head -n 1 | cut -d' ' -f2)
  git -C "$dir" rev-list --first-parent --max-count=11 "$oldest" | tail -n +2 |
    while read -r head; do
      jq -n --arg head "$head" --arg base "$(git -C "$dir" rev-parse "$head^1")" '{head: $head, base: $base}'
    done | jq -s --arg lang "$lang" --arg name "$name" --arg start "$oldest" '{language: $lang, fullName: $name, start: $start, changes: .}'
done | jq -s '{rule: "ten first-parent commits before the oldest sample base", repositories: .}' > "$here/selection-holdout.json"
jq -r '.repositories[] | "\(.fullName) \(.changes | length)"' "$here/selection-holdout.json"
