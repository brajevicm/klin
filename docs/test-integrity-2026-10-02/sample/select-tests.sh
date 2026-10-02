#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: select-tests.sh CLONES}

test_path() {
  case "$1" in
    *.rs) return 0 ;;
  esac
  case "/$1" in
    */test/* | */tests/* | */__tests__/* | */spec/* | */specs/* | */testing/* | */e2e/* | */__mocks__/* | */mocks/*) return 0 ;;
  esac
  case "${1##*/}" in
    *.test.* | *.spec.*) return 0 ;;
  esac
  return 1
}

qualifies() {
  local dir=$1 commit=$2 status file
  while IFS=$'\t' read -r status file; do
    [ "$status" = M ] || continue
    [[ "$file" =~ \.(rs|ts|mts|cts|tsx)$ ]] || continue
    test_path "$file" || continue
    if git -C "$dir" diff -U0 "$commit^" "$commit" -- "$file" | grep -E '^[-+]' | grep -vE '^(\+\+\+|---) ' | grep -qE 'assert|expect\('; then
      return 0
    fi
  done < <(git -C "$dir" diff --name-status "$commit^" "$commit")
  return 1
}

{
  echo '{"repositories":['
  first=true
  while read -r name start; do
    dir=$clones/${name//\//__}
    picked=()
    while read -r commit; do
      git -C "$dir" rev-parse --verify --quiet "$commit^" > /dev/null || continue
      if qualifies "$dir" "$commit"; then
        picked+=("$commit")
        [ ${#picked[@]} -lt 10 ] || break
      fi
    done < <(git -C "$dir" rev-list --first-parent "$start")
    $first || echo ','
    first=false
    printf '{"fullName":"%s","start":"%s","changes":[' "$name" "$start"
    sep=
    for commit in "${picked[@]}"; do
      printf '%s{"base":"%s","head":"%s"}' "$sep" "$(git -C "$dir" rev-parse "$commit^")" "$commit"
      sep=,
    done
    printf ']}'
  done < <(jq -r '.repositories[] | "\(.fullName) \(.start)"' "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json")
  echo ']}'
} | jq . > "$here/selection-tests.json"
jq -r '.repositories[] | "\(.fullName)\t\(.changes | length)"' "$here/selection-tests.json"
