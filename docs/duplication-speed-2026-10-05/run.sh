#!/bin/sh
# Usage: run.sh NAME ROOT [K W CAP MINIMIZER_T KEY_BITS SELECTION PHASE]. Builds the index cold, then runs 5 warm queries for 20 and 100 changed files at T=60.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
bin="$here/proto/target/release/dup-speed"
name=$1 root=$2 work=${WORK:-/tmp/dup-speed}
results=${RESULTS:-"$here/results-rerun"}
mkdir -p "$work" "$results"
out="$results/$name.jsonl"
: > "$out"
rss() { awk '/maximum resident/ {print $1}' "$work/time"; }
/usr/bin/time -l "$bin" build "$root" "$work/$name" ${3:-41} ${4:-20} ${5:-64} ${6:-5} ${7:-64} > "$work/q" 2>"$work/time"
sed "s/}\$/,\"kind\":\"build\",\"rss\":$(rss)}/" "$work/q" >> "$out"
for n in 20 100; do
  for run in 1 2 3 4 5; do
    /usr/bin/time -l "$bin" query "$root" "$work/$name" $n 60 ${8:-spread} ${9:-stop} > "$work/q" 2>"$work/time"
    sed "s/}\$/,\"kind\":\"query\",\"run\":$run,\"requested_changed\":$n,\"rss\":$(rss)}/" "$work/q" >> "$out"
  done
done
