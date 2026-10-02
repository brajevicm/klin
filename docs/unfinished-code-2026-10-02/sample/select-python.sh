#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
clones=${1:?usage: select-python.sh CLONES}
cutoff=2026-09-29T00:00:00Z
mkdir -p "$clones"

picked=()
skipped=()
while read -r name branch; do
  [ ${#picked[@]} -ge 5 ] && break
  dir=$clones/${name//\//__}
  [ -d "$dir" ] || git clone -q --single-branch --branch "$branch" "https://github.com/$name.git" "$dir" 2> /dev/null || {
    skipped+=("{\"fullName\":\"$name\",\"rule\":\"clone failed\"}")
    continue
  }
  start=$(git -C "$dir" log --first-parent --before="$cutoff" -n 1 --format=%H "origin/$branch")
  if [ -z "$start" ]; then
    skipped+=("{\"fullName\":\"$name\",\"rule\":\"no commit before the cutoff\"}")
    continue
  fi
  if ! git -C "$dir" cat-file -e "$start:pyproject.toml" 2> /dev/null &&
    ! git -C "$dir" cat-file -e "$start:setup.py" 2> /dev/null &&
    ! git -C "$dir" cat-file -e "$start:setup.cfg" 2> /dev/null; then
    skipped+=("{\"fullName\":\"$name\",\"rule\":\"no pyproject.toml, setup.py or setup.cfg at the root\"}")
    continue
  fi
  if git -C "$dir" cat-file -e "$start:klin.json" 2> /dev/null; then
    skipped+=("{\"fullName\":\"$name\",\"rule\":\"klin.json at the root\"}")
    continue
  fi
  commits=$(git -C "$dir" log --first-parent --format=%H -n 11 "$start")
  if [ "$(echo "$commits" | wc -l | tr -d ' ')" -lt 11 ]; then
    skipped+=("{\"fullName\":\"$name\",\"rule\":\"fewer than ten changes\"}")
    continue
  fi
  changes=$(echo "$commits" | awk 'NR>1 { printf "%s{\"head\":\"%s\",\"base\":\"%s\"}", (NR>2?",":""), prev, $1 } { prev=$1 }')
  picked+=("{\"language\":\"Python\",\"fullName\":\"$name\",\"defaultBranch\":\"$branch\",\"start\":\"$start\",\"changes\":[$changes]}")
done < <(jq -r '.items[] | "\(.full_name) \(.default_branch)"' "$here/search-python.json")

{
  printf '{"cutoff":"%s","repositories":[%s],"skipped":[%s]}\n' "$cutoff" "$(IFS=,; echo "${picked[*]:-}")" "$(IFS=,; echo "${skipped[*]:-}")"
} | jq . > "$here/selection-python.json"
jq -r '.repositories[].fullName, "skipped:", (.skipped[] | "\(.fullName): \(.rule)")' "$here/selection-python.json"
