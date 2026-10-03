#!/usr/bin/env bash
# cost.sh CLONES: five runs of each recipe over the whole tree and over 20
# files, at a sample repository's start commit. cost-runs.tsv keeps every run
# with its exit status and whether its SARIF report is a complete measurement:
# at least one run, no run with executionSuccessful false, and no error-level
# notification. cost.tsv holds the median of the complete runs, or "-" when
# fewer than three are. With RECIPE_ONLY=ruff-review it times only that recipe.
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
clones=${1:?usage: cost.sh CLONES}
runs=$here/cost-runs${RECIPE_ONLY:+-$RECIPE_ONLY}.tsv

now() { perl -MTime::HiRes=time -e 'printf "%.0f", time * 1000'; }

# One run: prints "exit<TAB>report<TAB>ms".
timed() {
  local tool=$1 start end code=0 report=no
  shift
  rm -f ".klin-recipes/$tool.sarif"
  start=$(now)
  sh "$here/recipes/run.sh" "$tool" "$tools" "$here/recipes" "$@" > /dev/null 2>&1 || code=$?
  end=$(now)
  jq -e '(.runs | type == "array" and length > 0)
    and ([.runs[].invocations[]? | select(.executionSuccessful == false)] | length == 0)
    and ([.runs[].invocations[]? | (.toolExecutionNotifications[]?, .toolConfigurationNotifications[]?) | select(.level == "error")] | length == 0)' \
    ".klin-recipes/$tool.sarif" > /dev/null 2>&1 && report=yes
  printf '%s\t%s\t%s\n' "$code" "$report" "$((end - start))"
}

median() {
  awk -F'\t' '$2 == "yes" { print $3 }' | sort -n |
    awk '{ v[NR] = $1 } END { if (NR < 3) print "-"; else print v[int((NR + 1) / 2)] }'
}

measure() {
  local name=$1 pattern=$2 start dir tool scope i row files=()
  shift 2
  dir=$clones/${name//\//__}
  start=$(jq -r --arg n "$name" '.repositories[] | select(.fullName == $n) | .start' \
    "$here/../../benchmark/evidence/false-alarms-2026-09-29/selection.json" "$here/../unfinished-code-2026-10-02/sample/selection-python.json")
  git -C "$dir" checkout -q --force --detach "$start"
  while read -r file; do files+=("$file"); done < <(git -C "$dir" ls-files | grep -E "$pattern" | grep -vE '(^|/)(tests?|__tests__)/' | head -n 20)
  for tool in "$@"; do
    for scope in whole 20; do
      [ "$tool" = gitleaks ] && [ "$scope" = 20 ] && continue
      for i in 1 2 3 4 5; do
        if [ "$scope" = whole ]; then
          row=$(cd "$dir" && timed "$tool")
        else
          row=$(cd "$dir" && timed "$tool" "${files[@]}")
        fi
        printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$tool" "$scope" "$i" "$row" >> "$runs"
      done
    done
    printf '%s\t%s\t%s\t%s\n' "$name" "$tool" \
      "$(awk -F'\t' -v n="$name" -v t="$tool" '$1 == n && $2 == t && $3 == "whole" { print $5 "\t" $6 "\t" $7 }' "$runs" | median)" \
      "$(awk -F'\t' -v n="$name" -v t="$tool" '$1 == n && $2 == t && $3 == "20" { print $5 "\t" $6 "\t" $7 }' "$runs" | median)"
  done
}

printf 'repository\ttool\tscope\trun\texit\treport\tms\n' > "$runs"
printf 'repository\ttool\twhole ms\t20 files ms\n'
if [ "${RECIPE_ONLY:-}" = ruff-review ]; then
  for name in teng-lin/notebooklm-py kvcache-ai/ktransformers; do
    measure "$name" '\.pyi?$' ruff-review
  done
  exit 0
fi
measure refactoringhq/tolaria '\.(ts|tsx|mts|cts)$' eslint eslint-injection semgrep gitleaks
for name in whyour/qinglong apollographql/apollo-client Open-Dev-Society/OpenStock mountain-loop/yaak; do
  measure "$name" '\.(ts|tsx|mts|cts)$' eslint-injection
done
measure teng-lin/notebooklm-py '\.pyi?$' ruff semgrep gitleaks
