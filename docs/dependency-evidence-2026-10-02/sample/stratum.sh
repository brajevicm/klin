#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: stratum.sh CLONES}
deps=$here/../prototype/deps.py
manifests='(^|/)(package\.json|Cargo\.toml|go\.mod|pyproject\.toml|requirements[^/]*\.txt|requirements/[^/]*\.in|setup\.cfg|setup\.py|Pipfile)$'
context='(^|/)(\.npmrc|\.env|Makefile)$|^\.github/workflows/'

export_manifests() {
  local paths
  rm -rf "$3"
  mkdir -p "$3"
  paths=$(git -C "$1" ls-tree -r --name-only "$2" | grep -E "$manifests|$context" || true)
  [ -n "$paths" ] || return 0
  tr '\n' '\0' <<< "$paths" | xargs -0 git -C "$1" archive "$2" -- | tar -x -C "$3"
}

touches_manifest() {
  git -C "$1" diff --name-only "$2" "$3" | grep -qE "$manifests"
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
entries=()
while read -r name start; do
  dir=$clones/${name//\//__}
  picked=()
  while read -r head base; do
    [ ${#picked[@]} -ge 10 ] && break
    [ -n "$base" ] || continue
    touches_manifest "$dir" "$base" "$head" || continue
    export_manifests "$dir" "$base" "$work/base"
    export_manifests "$dir" "$head" "$work/head"
    [ -n "$(python3 "$deps" added "$work/base" "$work/head")" ] || continue
    picked+=("{\"head\":\"$head\",\"base\":\"$base\"}")
  done < <(git -C "$dir" log --first-parent --format='%H %P' -n 2000 "$start" | awk '{ print $1, $2 }')
  entries+=("{\"fullName\":\"$name\",\"start\":\"$start\",\"changes\":[$(IFS=,; echo "${picked[*]:-}")]}")
  echo "$name: ${#picked[@]}" >&2
done < <(jq -r '.repositories[] | "\(.fullName) \(.start)"' \
  "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json")
printf '{"rule":"10 most recent first-parent commits at or before start with a new direct dependency, within 2000","repositories":[%s]}\n' \
  "$(IFS=,; echo "${entries[*]}")" | jq . > "$here/stratum.json"
