#!/usr/bin/env bash
# Exercise all's exit cleanup without starting an agent.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
out="$scratch/output with spaces"
while IFS=$'\t' read -r name _; do
  [ "$name" != case ] || continue
  for rep in 1 2; do
    mkdir -p "$out/cursor/$name-$rep"
    printf 'already completed\n' > "$out/cursor/$name-$rep/reply.txt"
  done
done < "$here/cases.tsv"
bash "$here/probe.sh" all cursor "$out"
[ ! -e "$out/.all-running" ]
