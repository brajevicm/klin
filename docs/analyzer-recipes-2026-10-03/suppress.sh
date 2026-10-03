#!/usr/bin/env bash
# suppress.sh: run each suppression route with the tool's inline suppressions
# honored, and again with the flag that turns them off.
set -u
C=$(cd "$(dirname "$0")" && pwd); R=$C/recipes; T=${TOOLS:?TOOLS names the directory of the pinned tools}; S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
run() { # lang route tool extra...
  local lang=$1 route=$2 tool=$3; shift 3
  W=$S/$lang-$route-$tool; rm -rf $W; mkdir -p $W; cp -R $C/fixtures/recipes-$lang/base/. $W; cp -R $C/fixtures/recipes-$lang/$route/. $W
  cd $W
  case $tool in
    eslint) $T/js/node_modules/.bin/eslint --no-config-lookup -c $T/js/klin-recipe.config.mjs -f json "$@" src > out.json; jq -c '[.[].messages[] | .ruleId]' out.json ;;
    ruff) $T/py/.venv/bin/ruff check --isolated --no-cache --exit-zero --select S307,S608,BLE001,S110,T201 --output-format json "$@" . | jq -c '[.[].code]' ;;
    semgrep) $T/py/.venv/bin/semgrep scan --config $R/semgrep.yml --metrics=off --disable-version-check --quiet --json "$@" . | jq -c '[.results[].check_id | sub("^.*\\.recipes\\."; "")]' ;;
    gitleaks) $T/gitleaks dir . --no-banner --exit-code 0 --report-format json --report-path out.json "$@" 2>/dev/null; jq -c '[.[].RuleID]' out.json ;;
  esac
}
for x in "ts inj-suppress-eslint eslint --no-inline-config" "ts debug-suppress-eslint eslint --no-inline-config" "ts debug-suppress-inline-config eslint --no-inline-config" "ts err-suppress-eslint eslint --no-inline-config" "py inj-suppress-noqa ruff --ignore-noqa" "py inj-suppress-file ruff --ignore-noqa" "py err-suppress-noqa ruff --ignore-noqa" "py debug-suppress-noqa ruff --ignore-noqa" "py inj-suppress-nosemgrep semgrep --disable-nosem" "ts inj-suppress-nosemgrep semgrep --disable-nosem" "ts sec-suppress-gitleaks gitleaks --ignore-gitleaks-allow" "py sec-suppress-gitleaks gitleaks --ignore-gitleaks-allow"; do
  set -- $x; echo "$1 $2 $3: honored=$(run $1 $2 $3) off=$(run $1 $2 $3 $4)"
done
