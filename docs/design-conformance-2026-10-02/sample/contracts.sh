#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: contracts.sh CLONES PILOT_CLONES}
pilot=${2:?usage: contracts.sh CLONES PILOT_CLONES}

trees() {
  jq -r '.repositories[] | "ordinary \(.fullName) \(.start)"' "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json"
  jq -r '.agent[] | select(.language == "Rust" or .language == "TypeScript") | "agent \(.fullName) \(.head)"' \
    "$repo/docs/phenotype-pilot-2026-10-02/selection.json" | sort -u
}

rule() {
  local dir=$1 commit=$2 path=$3 name=${3##*/}
  local body
  body=$(git -C "$dir" show "$commit:$path" 2> /dev/null || true)
  case "$name" in
    klin.json) grep -q '"layering"' <<< "$body" && echo "klin-layering" ;;
    .dependency-cruiser.js | .dependency-cruiser.cjs | .dependency-cruiser.mjs | .dependency-cruiser.json) echo "dependency-cruiser" ;;
    .eslintrc* | eslint.config.*)
      grep -oE 'no-restricted-imports|no-restricted-syntax|import/no-restricted-paths|boundaries/[a-z-]+|@nx/enforce-module-boundaries' <<< "$body" | sort -u | sed 's/^/eslint:/' ;;
    clippy.toml | .clippy.toml) grep -oE 'disallowed-(methods|types|macros)' <<< "$body" | sort -u | sed 's/^/clippy:/' ;;
    deny.toml) grep -q '^\[bans\]' <<< "$body" && echo "cargo-deny-bans" ;;
    .importlinter) echo "import-linter" ;;
    pyproject.toml | setup.cfg) grep -q 'importlinter' <<< "$body" && echo "import-linter" ;;
  esac
}

: > "$here/contracts.tsv"
while read -r set name commit; do
  dir=$clones/${name//\//__}
  [ "$set" = ordinary ] || dir=$pilot/${name//\//__}
  found=0
  while read -r path; do
    case "$path" in */node_modules/* | node_modules/* | */vendor/*) continue ;; esac
    while read -r kind; do
      [ -n "$kind" ] || continue
      printf '%s\t%s\t%s\t%s\t%s\n' "$set" "$name" "$commit" "$path" "$kind" >> "$here/contracts.tsv"
      found=1
    done < <(rule "$dir" "$commit" "$path")
  done < <(git -C "$dir" ls-tree -r --name-only "$commit" | grep -E '(^|/)(klin\.json|\.dependency-cruiser\.(js|cjs|mjs|json)|\.eslintrc[^/]*|eslint\.config\.[^/]+|\.?clippy\.toml|deny\.toml|\.importlinter|pyproject\.toml|setup\.cfg)$' || true)
  [ "$found" = 1 ] || printf '%s\t%s\t%s\t-\tnone\n' "$set" "$name" "$commit" >> "$here/contracts.tsv"
done < <(trees)
