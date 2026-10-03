#!/usr/bin/env bash
# drift.sh CLONES OLD NEW: run each recipe at every sample start commit under
# two tool directories and count the results each version alone reports. A
# result is its rule, file, line and message. ESLint runs over the first 200
# tracked TypeScript files outside tests, because ESLint 9.30.0 ran for more
# than 15 minutes over tolaria's whole tree. Each run stops at 300 seconds, and
# a run that stops there counts as "-".
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
clones=${1:?usage: drift.sh CLONES OLD NEW}
old=${2:?}
new=${3:?}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

keys() {
  jq -r '.runs[].results[] | [(.ruleId // "-" | sub("^.*\\.recipes\\."; "")),
    (.locations[0].physicalLocation.artifactLocation.uri // "-" | sub("^file://.*/sample/[^/]+/"; "")),
    (.locations[0].physicalLocation.region.startLine // 0 | tostring), (.message.text | gsub("[\t\n]"; " "))] | @tsv' "$1" 2> /dev/null | sort
}

printf 'repository\ttool\told\tnew\tonly old\tonly new\n'
jq -r '.repositories[] | select(.language != "Rust") | "\(.language) \(.fullName) \(.start)"' \
  "$here/../../benchmark/evidence/false-alarms-2026-09-29/selection.json" "$here/../unfinished-code-2026-10-02/sample/selection-python.json" |
while read -r lang name start; do
  dir=$clones/${name//\//__}
  git -C "$dir" checkout -q --force --detach "$start"
  case $lang in TypeScript) list="eslint semgrep gitleaks" ;; Python) list="ruff semgrep gitleaks" ;; esac
  files=()
  while read -r file; do files+=("$file"); done < <(git -C "$dir" ls-files | grep -E '\.(ts|tsx|mts|cts)$' | grep -vE '(^|/)(tests?|__tests__)/|\.d\.ts$' | head -n 200)
  for tool in $list; do
    scope=()
    [ "$tool" = eslint ] && scope=("${files[@]}")
    for side in old new; do
      tools=$old
      [ $side = new ] && tools=$new
      rm -f "$dir/.klin-recipes/$tool.sarif"
      code=0
      (cd "$dir" && perl -e 'alarm 300; exec @ARGV' sh "$here/recipes/run.sh" "$tool" "$tools" "$here/recipes" ${scope[@]+"${scope[@]}"} > /dev/null 2>&1) || code=$?
      if [ "$code" = 142 ]; then touch "$work/$side.stopped"; else rm -f "$work/$side.stopped"; fi
      keys "$dir/.klin-recipes/$tool.sarif" > "$work/$side" || true
    done
    if [ -f "$work/old.stopped" ] || [ -f "$work/new.stopped" ]; then
      printf '%s\t%s\t%s\t%s\t-\t-\n' "$name" "$tool" "$([ -f "$work/old.stopped" ] && echo - || wc -l < "$work/old" | tr -d ' ')" "$([ -f "$work/new.stopped" ] && echo - || wc -l < "$work/new" | tr -d ' ')"
      continue
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$tool" "$(wc -l < "$work/old")" "$(wc -l < "$work/new")" \
      "$(comm -23 "$work/old" "$work/new" | wc -l)" "$(comm -13 "$work/old" "$work/new" | wc -l)" | tr -d ' '
  done
done
