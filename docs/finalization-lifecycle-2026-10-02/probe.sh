#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
git_() { git -c user.email=p@invalid -c user.name=probe -c commit.gpgsign=false "$@"; }
tree_of() {
  local idx
  idx="$(git -C "$1" rev-parse --absolute-git-dir)/$2.idx"
  rm -f "$idx"
  (cd "$1" && GIT_INDEX_FILE=$idx git add -A && GIT_INDEX_FILE=$idx git write-tree)
}
hooks() {
  python3 -c 'import json,sys; b=sys.argv[1]; h=lambda e: [{"hooks": [{"type": "command", "command": json.dumps(b + "/hook")[1:-1] + " " + e}]}]; print(json.dumps({"hooks": {"SessionStart": h("session"), "Stop": h("stop")}}))' "$1"
}
cursor_hooks() {
  python3 -c 'import json,sys; c=lambda e: [{"command": sys.argv[1] + "/hook " + e}]; print(json.dumps({"version": 1, "hooks": {"sessionStart": c("session"), "stop": c("stop")}}))' "$1"
}
stage() {
  local tree=$1 host=${2:-agents}
  mkdir -p "$tree"; cp -R "$here/fixture/." "$tree/"
  if [ "$host" = cursor ]; then
    mkdir -p "$tree/.cursor" "$tree/.vscode"
    cursor_hooks "$here/bin" > "$tree/.cursor/hooks.json"
    python3 -c 'import json,sys; print(json.dumps({"terminal.integrated.env.osx": {"PATH": sys.argv[1] + ":${env:PATH}"}}))' "$here/bin" > "$tree/.vscode/settings.json"
  else
    mkdir -p "$tree/.claude" "$tree/.codex"
    hooks "$here/bin" > "$tree/.claude/settings.json"
    hooks "$here/bin" > "$tree/.codex/hooks.json"
  fi
  [ "$host" != codex ] || cp "$here/context.txt" "$tree/AGENTS.md"
  printf '__pycache__/\n' > "$tree/.gitignore"
  git_ -C "$tree" init -q -b main
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -qm base
}
cursor() {
  local name=$1 out=$2 mode
  local dir="$out/cursor/$name-1"
  mode=$(awk -F'\t' -v c="$name" '$1 == c { print $2 }' "$here/cases.tsv")
  [ -n "$mode" ] || { echo "probe: no case $name in cases.tsv" >&2; exit 2; }
  rm -rf "$dir"; mkdir -p "$dir"; stage "$dir/tree" cursor
  : > "$dir/probe.log"
  printf 'PROBE_MODE=%q\nPROBE_LOG=%q\nPROBE_CONTEXT=%q\n' "$mode" "$dir/probe.log" "$here/context.txt" \
    > "$(git -C "$dir/tree" rev-parse --absolute-git-dir)/probe.env"
  echo "$dir/tree"
}
run() {
  local host=$1 name=$2 rep=$3 out=$4
  local dir="$out/$host/$name-$rep" mode task
  mode=$(awk -F'\t' -v c="$name" '$1 == c { print $2 }' "$here/cases.tsv")
  task=$(awk -F'\t' -v c="$name" '$1 == c { print $3 }' "$here/cases.tsv")
  [ -n "$task" ] || { echo "probe: no case $name in cases.tsv" >&2; exit 2; }
  rm -rf "$dir"; mkdir -p "$dir"; stage "$dir/tree" "$host"
  local context="$here/context.txt"
  if [ "$host" = cursor ]; then
    context="$dir/context.txt"
    sed "s|\`klin finalize\`|\`$here/bin/klin finalize\`|" "$here/context.txt" > "$context"
  fi
  export PATH="$here/bin:$PATH" PROBE_MODE="$mode" PROBE_LOG="$dir/probe.log" PROBE_CONTEXT="$context"
  : > "$dir/probe.log"
  printf 'PROBE_MODE=%q\nPROBE_LOG=%q\nPROBE_CONTEXT=%q\n' "$mode" "$dir/probe.log" "$context" \
    > "$(git -C "$dir/tree" rev-parse --absolute-git-dir)/probe.env"
  case $host in
    claude) (cd "$dir/tree" && claude -p "$task" --model sonnet --setting-sources project \
        --strict-mcp-config --dangerously-skip-permissions --output-format json) \
        > "$dir/reply.json" 2> "$dir/host.err" || true
      python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("result",""))' "$dir/reply.json" > "$dir/reply.txt" || true ;;
    codex) codex exec --ignore-user-config --dangerously-bypass-hook-trust -s workspace-write \
        -m gpt-6.1-sol -c model_reasoning_effort=low -c allow_login_shell=false -C "$dir/tree" -o "$dir/reply.txt" "$task" \
        > "$dir/host.out" 2> "$dir/host.err" < /dev/null || true ;;
    cursor) (cd "$dir/tree" && cursor-agent -p --force --trust --output-format json \
        ${PROBE_CURSOR_MODEL:+--model "$PROBE_CURSOR_MODEL"} --workspace "$dir/tree" "$task") \
        > "$dir/reply.json" 2> "$dir/host.err" < /dev/null || true
      python3 -c 'import json,sys; o=json.load(open(sys.argv[1])); print(o.get("result") or o.get("text") or "")' "$dir/reply.json" > "$dir/reply.txt" || true ;;
    *) echo "probe: HOST is claude, codex or cursor" >&2; exit 2 ;;
  esac
  record "$dir"
}
record() {
  git -C "$1/tree" diff main -- . ':!.claude' ':!.codex' ':!.cursor' ':!.vscode' ':!AGENTS.md' > "$1/change.diff" || true
  git -C "$1/tree" status --porcelain --untracked-files=all >> "$1/change.diff" || true
}
all() {
  local host=$1 out=$2 name rep
  mkdir -p "$out"
  mkdir "$out/.all-running" 2>/dev/null || { echo "probe: another all runs in $out" >&2; exit 2; }
  trap "rmdir -- $(printf '%q' "$out/.all-running")" EXIT
  for name in $(awk -F'\t' 'NR > 1 { print $1 }' "$here/cases.tsv"); do
    for rep in 1 2; do
      [ -s "$out/$host/$name-$rep/reply.txt" ] || run "$host" "$name" "$rep" "$out"
    done
  done
}
score() {
  local out=$1 dir host run_name final base
  printf 'host\tcase\trep\tfinalize_calls\tverdicts\tstops\tchanged\tfinal_tree_is_last_finalize_tree\ttree_changed_after_first_finalize\n'
  for dir in "$out"/*/*; do
    [ -f "$dir/probe.log" ] || continue
    host=$(basename "$(dirname "$dir")"); run_name=$(basename "$dir")
    final=$(tree_of "$dir/tree" score)
    base=$(git -C "$dir/tree" rev-parse 'main^{tree}')
    awk -F'\t' -v host="$host" -v cr="$run_name" -v final="$final" -v base="$base" '
      $1 == "finalize" { n++; v = v (v ? "," : "") $6; last = $5; if (!first) first = $5 }
      $1 == "stop" { s++ }
      END {
        c = cr; sub(/-[0-9]+$/, "", c); r = cr; sub(/^.*-/, "", r)
        printf "%s\t%s\t%s\t%d\t%s\t%d\t%s\t%s\t%s\n", host, c, r, n, (v ? v : "-"), s,
          (final != base ? "yes" : "no"), (n ? (final == last ? "yes" : "no") : "-"),
          (n ? (final != first ? "yes" : "no") : "-")
      }' "$dir/probe.log"
  done
}
"$@"
