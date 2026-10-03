#!/usr/bin/env bash
# replay.sh CLONES OUT [whole]: run the recipes of each sample change through
# `klin gate --strict --json`, the way CI judges a pull request, and write
# failures.tsv, gates.tsv and changes.tsv into OUT. With `whole`, ESLint, Ruff
# and Semgrep run over the whole tree instead of the changed files.
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
corpus=$(cd "$here/.." && pwd)
repo=$(cd "$corpus/../.." && pwd)
klin=${KLIN:-$repo/target/release/klin}
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
clones=${1:?usage: replay.sh CLONES OUT [whole]}
out=${2:?usage: replay.sh CLONES OUT [whole]}
scope=${3:-changed}
only=${ONLY:-}
home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT
mkdir -p "$out/json"

quoted() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }

changes() {
  jq -r '.repositories[] | select(.language != "Rust") | .language as $lang | .fullName as $name | .changes[] | "\($lang) \($name) \(.base) \(.head)"' \
    "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$repo/docs/unfinished-code-2026-10-02/sample/selection-python.json"
}

config() {
  local tools_of=$1 tool entry list file
  shift
  list=
  for file in "$@"; do list="$list $(quoted "$file")"; done
  for tool in $tools_of; do
    entry="sh $corpus/recipes/run.sh $tool $tools $corpus/recipes"
    [ "$tool" = gitleaks ] || [ "$scope" = whole ] || entry="$entry$list"
    jq -n --arg name "$tool" --arg run "$entry" --arg report ".klin-recipes/$tool.sarif" '{name: $name, run: $run, report: $report}'
  done | jq -s '{sarif: .}'
}

: > "$out/failures.tsv"
: > "$out/gates.tsv"
: > "$out/changes.tsv"
while read -r lang name base head; do
  [ -z "$only" ] || [ "$only" = "$head" ] || continue
  dir=$clones/${name//\//__}
  case $lang in
    TypeScript) pattern='\.(ts|tsx|mts|cts)$' tools_of="eslint semgrep gitleaks" ;;
    Python) pattern='\.(py|pyi)$' tools_of="ruff semgrep gitleaks" ;;
  esac
  git -C "$dir" checkout -q --force --detach "$head"
  git -C "$dir" clean -fdq
  git -C "$dir" branch -q -f klin-base "$base"
  grep -qx '.klin-recipes/' "$dir/.git/info/exclude" 2> /dev/null || printf '.klin-recipes/\nklin.json\n' >> "$dir/.git/info/exclude"
  files=()
  while read -r file; do files+=("$file"); done < <(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head" | grep -E "$pattern" || true)
  printf '%s\t%s\t%s\t%s\n' "$lang" "$name" "$head" "${#files[@]}" >> "$out/changes.tsv"
  [ ${#files[@]} -gt 0 ] || continue
  config "$tools_of" "${files[@]}" > "$dir/klin.json"
  gates=()
  for tool in $tools_of; do gates+=(--gate "$tool"); done
  json=$out/json/${name//\//__}-${head:0:10}.json
  (cd "$dir" && env -i PATH="$PATH" HOME="$home" GITHUB_BASE_REF=klin-base "$klin" gate --strict --json "${gates[@]}") > "$json" 2> "$json.err" || true
  jq -r --arg lang "$lang" --arg name "$name" --arg head "${head:0:10}" \
    '.findings[]? | [$lang, $name, $head, .gate, .file, (.line // 0 | tostring), (.text | split(": ")[0] | sub("^.*\\.recipes\\."; "")), (.text | split(": ")[1:] | join(": ") | gsub("[\t\n]"; " "))] | @tsv' "$json" >> "$out/failures.tsv"
  jq -r --arg lang "$lang" --arg name "$name" --arg head "${head:0:10}" \
    '.gates[]? | [$lang, $name, $head, .name, .status, (.ms | tostring), (.findings | tostring), (.held | tostring)] | @tsv' "$json" >> "$out/gates.tsv"
  rm -f "$dir/klin.json"
done < <(changes)
wc -l "$out/changes.tsv" "$out/failures.tsv" "$out/gates.tsv"
