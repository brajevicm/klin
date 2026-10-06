#!/bin/sh
# Use prepared pinned corpus roots; run.sh owns timing and resource accounting.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
corpus=$1
results=${RESULTS:-"$here/results-rerun"}
export RESULTS="$results"
python3 "$here/workloads.py" "$corpus"
for name in 10k 300k glaredb karakeep; do
  "$here/run.sh" "$name-64" "$corpus/$name" 41 20 64 5 64 spread stop
done
for selection in spread largest; do
  "$here/run.sh" "klin-$selection-64" "$corpus/klin/src" 41 20 64 5 64 "$selection" stop
done
for bits in 32 48 64; do
  "$here/run.sh" "1m-$bits" "$corpus/1m" 41 20 64 5 "$bits" spread stop
done
"$here/run.sh" 1m-largest-64 "$corpus/1m" 41 20 64 5 64 largest stop
"$here/run.sh" changed-100k "$corpus/changed-100k" 41 20 64 5 64 spread stop
for multiplicity in 2 10 40 63 64 65 100 1000; do
  "$here/run.sh" "multiplicity-$multiplicity" "$corpus/multiplicity-$multiplicity" 41 20 64 5 64 spread stop
done
python3 "$here/summary.py" "$results" > "$results/summary.txt"
