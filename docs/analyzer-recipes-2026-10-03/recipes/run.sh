#!/bin/sh
# run.sh TOOL TOOLS RECIPES [FILE...]: write .klin-recipes/TOOL.sarif over the
# named files, or over the working tree when none are named.
set -u
tool=$1 tools=$2 recipes=$3
shift 3
[ $# -gt 0 ] || set -- .
mkdir -p .klin-recipes
case $tool in
  eslint)
    exec "$tools/js/node_modules/.bin/eslint" --no-config-lookup -c "$tools/js/klin-recipe.config.mjs" \
      -f "$tools/js/node_modules/@microsoft/eslint-formatter-sarif/sarif.js" -o .klin-recipes/eslint.sarif \
      --no-warn-ignored "$@" ;;
  eslint-injection)
    exec "$tools/js/node_modules/.bin/eslint" --no-config-lookup -c "$tools/js/klin-recipe-injection.config.mjs" \
      -f "$tools/js/node_modules/@microsoft/eslint-formatter-sarif/sarif.js" -o .klin-recipes/eslint-injection.sarif \
      --no-warn-ignored "$@" ;;
  ruff)
    exec "$tools/py/.venv/bin/ruff" check --isolated --no-cache --exit-zero \
      --select "$(awk -F'\t' '$1 == "ruff" { printf "%s%s", sep, $2; sep = "," }' "$recipes/families.tsv")" \
      --output-format sarif -o .klin-recipes/ruff.sarif "$@" ;;
  ruff-review)
    exec "$tools/py/.venv/bin/ruff" check --isolated --no-cache --exit-zero --ignore-noqa \
      --select S102,S307,S602,S604,S605,S608,BLE001,S110,S112 \
      --output-format sarif -o .klin-recipes/ruff-review.sarif "$@" ;;
  semgrep)
    exec "$tools/py/.venv/bin/semgrep" scan --config "$recipes/semgrep.yml" --metrics=off --disable-version-check \
      --quiet --sarif --output .klin-recipes/semgrep.sarif "$@" ;;
  gitleaks)
    exec "$tools/gitleaks" dir . --no-banner --exit-code 0 --report-format sarif \
      --report-path .klin-recipes/gitleaks.sarif ;;
esac
echo "unknown tool $tool" >&2
exit 2