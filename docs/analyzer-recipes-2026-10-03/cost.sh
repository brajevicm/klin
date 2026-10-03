#!/usr/bin/env bash
# cost.sh CLONES: median of 5 runs of each recipe over the whole tree and over
# 20 changed files, on the largest sample repository of each language, at its
# start commit.
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
clones=${1:?usage: cost.sh CLONES}

median() { sort -n | awk '{ v[NR] = $1 } END { print v[int((NR + 1) / 2)] }'; }

timed() {
  local start end
  start=$(perl -MTime::HiRes=time -e 'printf "%.0f", time * 1000')
  sh "$here/recipes/run.sh" "$@" > /dev/null 2>&1 || true
  end=$(perl -MTime::HiRes=time -e 'printf "%.0f", time * 1000')
  echo $((end - start))
}

measure() {
  local name=$1 pattern=$2 start dir tool i files=()
  shift 2
  dir=$clones/${name//\//__}
  start=$(jq -r --arg n "$name" '.repositories[] | select(.fullName == $n) | .start' \
    "$here/../../benchmark/evidence/false-alarms-2026-09-29/selection.json" "$here/../unfinished-code-2026-10-02/sample/selection-python.json")
  git -C "$dir" checkout -q --force --detach "$start"
  while read -r file; do files+=("$file"); done < <(git -C "$dir" ls-files | grep -E "$pattern" | grep -vE '(^|/)(tests?|__tests__)/' | head -n 20)
  for tool in "$@"; do
    whole=$(for i in 1 2 3 4 5; do (cd "$dir" && timed "$tool" "$tools" "$here/recipes"); done | median)
    if [ "$tool" = gitleaks ]; then
      changed=-
    else
      changed=$(for i in 1 2 3 4 5; do (cd "$dir" && timed "$tool" "$tools" "$here/recipes" "${files[@]}"); done | median)
    fi
    printf '%s\t%s\t%s\t%s\n' "$name" "$tool" "$whole" "$changed"
  done
}

printf 'repository\ttool\twhole ms\t20 files ms\n'
measure refactoringhq/tolaria '\.(ts|tsx|mts|cts)$' eslint semgrep gitleaks
measure teng-lin/notebooklm-py '\.pyi?$' ruff semgrep gitleaks
