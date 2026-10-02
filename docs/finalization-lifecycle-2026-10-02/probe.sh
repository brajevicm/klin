#!/usr/bin/env bash
# The #352 agent probe. One run lays a fresh tree, gives one agent one task
# with the proposed readiness line at session start, and keeps the logs.
#   probe.sh run HOST CASE REP OUT     HOST is claude or codex
#   probe.sh score OUT                 one TSV row per run under OUT
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
hooks() {
  local bin=$1
  printf '{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"%s/hook session"}]}],"Stop":[{"hooks":[{"type":"command","command":"%s/hook stop"}]}]}}\n' "$bin" "$bin"
}
stage() {
  local tree=$1
  mkdir -p "$tree"; cp -R "$here/fixture/." "$tree/"
  mkdir -p "$tree/.claude" "$tree/.codex"
  hooks "$here/bin" > "$tree/.claude/settings.json"
  hooks "$here/bin" > "$tree/.codex/hooks.json"
  printf '__pycache__/\n' > "$tree/.gitignore"
  git -C "$tree" init -q -b main
  git -C "$tree" -c user.email=p@invalid -c user.name=probe add -A
  git -C "$tree" -c user.email=p@invalid -c user.name=probe commit -qm base
}
run() {
  local host=$1 case=$2 rep=$3 out=$4
  local dir="$out/$host/$case-$rep" mode task
  mode=$(awk -F'\t' -v c="$case" '$1 == c { print $2 }' "$here/cases.tsv")
  task=$(awk -F'\t' -v c="$case" '$1 == c { print $3 }' "$here/cases.tsv")
  rm -rf "$dir"; mkdir -p "$dir"; stage "$dir/tree"
  export PATH="$here/bin:$PATH" PROBE_MODE="$mode" PROBE_LOG="$dir/probe.log" PROBE_CONTEXT="$here/context.txt"
  : > "$dir/probe.log"
  case $host in
    claude) (cd "$dir/tree" && claude -p "$task" --model sonnet --setting-sources project \
        --strict-mcp-config --dangerously-skip-permissions --output-format json) \
        > "$dir/reply.json" 2> "$dir/host.err" || true
      python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("result",""))' "$dir/reply.json" > "$dir/reply.txt" || true ;;
    codex) codex exec --ignore-user-config --dangerously-bypass-hook-trust -s workspace-write \
        -m gpt-6.1-sol -c model_reasoning_effort=low -C "$dir/tree" -o "$dir/reply.txt" "$task" \
        > "$dir/host.out" 2> "$dir/host.err" < /dev/null || true ;;
  esac
  git -C "$dir/tree" diff main -- . ':!.claude' ':!.codex' > "$dir/change.diff" || true
  git -C "$dir/tree" status --porcelain --untracked-files=all >> "$dir/change.diff" || true
}
score() {
  local out=$1
  printf 'host\tcase\trep\tfinalize_calls\tverdicts\tstops\tchanged\tfinal_tree_is_last_finalize_tree\ttree_changed_after_first_finalize\n'
  for dir in "$out"/*/*; do
    [ -f "$dir/probe.log" ] || continue
    local host case_rep log final idx
    host=$(basename "$(dirname "$dir")"); case_rep=$(basename "$dir"); log="$dir/probe.log"
    idx="$(git -C "$dir/tree" rev-parse --git-dir)/score.idx"; rm -f "$idx"
    final=$(cd "$dir/tree" && GIT_INDEX_FILE=$idx git add -A && GIT_INDEX_FILE=$idx git write-tree)
    base=$(git -C "$dir/tree" rev-parse 'main^{tree}')
    awk -F'\t' -v host="$host" -v cr="$case_rep" -v final="$final" -v base="$base" '
      $1 == "finalize" { n++; v = v (v ? "," : "") $6; last = $5; if (!first) first = $5 }
      $1 == "stop" { s++ }
      END {
        c = cr; sub(/-[0-9]+$/, "", c); r = cr; sub(/^.*-/, "", r)
        printf "%s\t%s\t%s\t%d\t%s\t%d\t%s\t%s\t%s\n", host, c, r, n, (v ? v : "-"), s,
          (final != base ? "yes" : "no"), (n ? (final == last ? "yes" : "no") : "-"),
          (n ? (final != first ? "yes" : "no") : "-")
      }' "$log"
  done
}
"$@"
