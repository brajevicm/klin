#!/usr/bin/env bash
# ruff-review.sh: the recipe-level probes of the exact Ruff recipe. For each
# Python suppression route, and for one plant per family, the findings under
# the shell seam, under env -i with an empty HOME, and under a sandbox that
# denies the network, plus every file the run wrote in the tree or in HOME.
set -u
here=$(cd "$(dirname "$0")" && pwd)
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
printf '(version 1)\n(allow default)\n(deny network-outbound (remote ip))\n(deny network-inbound (local ip))\n' > "$work/nonet.sb"
for route in inj-plant-sql inj-plant-exec inj-plant-eval inj-neg-const sec-plant-key sec-plant-password sec-plant-constant sec-neg-name \
  err-plant-pass dead-plant-variable inj-suppress-noqa inj-suppress-file inj-suppress-nosemgrep err-suppress-noqa debug-suppress-noqa; do
  tree=$work/$route
  mkdir -p "$tree"
  cp -R "$here/fixtures/recipes-py/base/." "$tree"
  cp -R "$here/fixtures/recipes-py/$route/." "$tree"
  line="$route"
  for mode in shell envi nonet; do
    home=$work/home-$route-$mode
    mkdir -p "$home"
    touch "$work/marker"
    sleep 1
    rm -f "$tree/.klin-recipes/ruff-review.sarif"
    case $mode in
      shell) (cd "$tree" && sh "$here/recipes/run.sh" ruff-review "$tools" "$here/recipes" > /dev/null 2>&1) ;;
      envi) (cd "$tree" && env -i PATH=/usr/bin:/bin HOME="$home" sh "$here/recipes/run.sh" ruff-review "$tools" "$here/recipes" > /dev/null 2>&1) ;;
      nonet) (cd "$tree" && env -i PATH=/usr/bin:/bin HOME="$home" sandbox-exec -f "$work/nonet.sb" sh "$here/recipes/run.sh" ruff-review "$tools" "$here/recipes" > /dev/null 2>&1) ;;
    esac
    found=$(jq -r '[.runs[].results[] | "\(.ruleId)@\(.locations[0].physicalLocation.artifactLocation.uri | sub(".*/" + "'"$route"'" + "/"; ""))"] | sort | join(" ")' "$tree/.klin-recipes/ruff-review.sarif" 2> /dev/null || echo noreport)
    writes=$( (cd "$tree" && find . -newer "$work/marker" -type f -not -path './.klin-recipes/*'; cd "$home" && find . -type f) | sort | tr '\n' ' ')
    line="$line	$mode: ${found:--} | writes: ${writes:--}"
  done
  printf '%s\n' "$line"
done
