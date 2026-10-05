#!/bin/sh
# Usage: run.sh NAME ROOT [K W CAP FCUT T]. Builds the index cold, then runs 5 warm queries for 20 and 100 changed files at T=60.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
bin="$here/proto/target/release/dup-speed"
name=$1 root=$2 work=${WORK:-/tmp/dup-speed}
mkdir -p "$work" "$here/results"
out="$here/results/$name.jsonl"
: > "$out"
rss() { awk '/maximum resident/ {print $1}' "$work/time"; }
/usr/bin/time -l "$bin" build "$root" "$work/$name" ${3:-41} ${4:-20} ${5:-64} 10 ${6:-98} ${7:-5} > "$work/q" 2>"$work/time"
sed "s/}\$/,\"kind\":\"build\",\"rss\":$(rss)}/" "$work/q" >> "$out"
for n in 20 100; do
  for run in 1 2 3 4 5; do
    /usr/bin/time -l "$bin" query "$root" "$work/$name" $n 60 > "$work/q" 2>"$work/time"
    sed "s/}\$/,\"kind\":\"query\",\"run\":$run,\"rss\":$(rss)}/" "$work/q" >> "$out"
  done
done
