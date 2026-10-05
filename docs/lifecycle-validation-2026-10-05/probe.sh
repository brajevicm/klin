#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
klin=${KLIN:-$repo/target/release/klin}
git_() { git -c user.email=p@invalid -c user.name=probe -c commit.gpgsign=false "$@"; }
q() { printf '%q' "$1"; }

nested_hooks() {
  python3 - "$1" <<'PY'
import json, sys
k = sys.argv[1]
h = lambda a, m=None: [dict({"hooks": [{"type": "command", "command": k + " " + a, "timeout": 600}]}, **({"matcher": m} if m else {}))]
print(json.dumps({"hooks": {"SessionStart": h("radius"), "UserPromptSubmit": h("radius"),
  "PreToolUse": h("guard", "Write|Edit|MultiEdit|Bash|apply_patch"), "Stop": h("gate --hook --changed")}}))
PY
}
cursor_hooks() {
  python3 - "$1" <<'PY'
import json, sys
k = sys.argv[1]
c = lambda a: [{"command": k + " " + a}]
print(json.dumps({"version": 1, "hooks": {"sessionStart": c("radius"), "beforeSubmitPrompt": c("radius"),
  "preToolUse": c("guard"), "beforeShellExecution": c("guard"), "stop": c("gate --hook --changed")}}))
PY
}

stage() {
  local tree=$1 host=$2
  mkdir -p "$tree"; cp -R "$here/fixture/." "$tree/"
  printf '__pycache__/\n' > "$tree/.gitignore"
  case $host in
    claude) mkdir -p "$tree/.claude"; nested_hooks "$(q "$klin")" > "$tree/.claude/settings.json" ;;
    cursor) mkdir -p "$tree/.cursor"; cursor_hooks "$(q "$klin")" > "$tree/.cursor/hooks.json" ;;
  esac
  git_ -C "$tree" init -q -b main
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -qm base
  git_ -C "$tree" checkout -qb work
}

codex_home() {
  mkdir -p "$1"
  ln -sf "$HOME/.codex/auth.json" "$1/auth.json"
  nested_hooks "$(q "$klin")" > "$1/hooks.json"
}

run() {
  local host=$1 name=$2 rep=$3 out=$4 task dir
  task=$(awk -F'\t' -v c="$name" '$1 == c { print $2 }' "$here/cases.tsv")
  [ -n "$task" ] || { echo "probe: no case $name" >&2; exit 2; }
  dir="$out/$host/$name-$rep"
  rm -rf "$dir"; mkdir -p "$dir"; stage "$dir/tree" "$host"
  "$klin" --version > "$dir/klin.version"
  case $host in
    claude) (cd "$dir/tree" && claude -p "$task" --model sonnet --setting-sources project \
        --strict-mcp-config --dangerously-skip-permissions --output-format json) \
        > "$dir/reply.json" 2> "$dir/host.err" < /dev/null || true
      python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("result",""))' "$dir/reply.json" > "$dir/reply.txt" || true ;;
    codex) codex_home "$dir/codex-home"
      CODEX_HOME="$dir/codex-home" codex exec --dangerously-bypass-hook-trust -s workspace-write \
        -m gpt-6.1-sol -c model_reasoning_effort=low -c allow_login_shell=false -C "$dir/tree" \
        -o "$dir/reply.txt" "$task" > "$dir/host.out" 2> "$dir/host.err" < /dev/null || true
      rm -f "$dir/codex-home/auth.json" ;;
    *) echo "probe: HOST is claude or codex; use cursor-stage for Cursor" >&2; exit 2 ;;
  esac
  record "$dir"
}

cursor-stage() {
  local name=$1 rep=$2 out=$3 dir
  dir="$out/cursor/$name-$rep"
  rm -rf "$dir"; mkdir -p "$dir"; stage "$dir/tree" cursor
  "$klin" --version > "$dir/klin.version"
  echo "open $dir/tree in the Cursor app, send this prompt, then run: probe.sh record $dir"
  awk -F'\t' -v c="$name" '$1 == c { print $2 }' "$here/cases.tsv"
}

record() {
  local dir=$1
  git -C "$dir/tree" diff main -- . ':!.claude' ':!.cursor' > "$dir/change.diff" || true
  git -C "$dir/tree" status --porcelain --untracked-files=all -- . ':!.claude' ':!.cursor' >> "$dir/change.diff" || true
  cp "$dir/tree/.git/klin/journal.jsonl" "$dir/journal.jsonl" 2> /dev/null || : > "$dir/journal.jsonl"
}

all() {
  local host=$1 out=$2
  for spec in clean:1 clarify-stub:1 clarify-stub:2 clarify-natural:1; do
    [ -s "$out/$host/${spec%:*}-${spec#*:}/reply.txt" ] || run "$host" "${spec%:*}" "${spec#*:}" "$out"
  done
}

score() {
  printf 'host\trun\tstops\tblocks\tlast_verdict\tchanged\n'
  for dir in "$1"/*/*; do
    [ -f "$dir/journal.jsonl" ] || continue
    jq -rs --arg h "$(basename "$(dirname "$dir")")" --arg r "$(basename "$dir")" \
      --arg c "$([ -s "$dir/change.diff" ] && echo yes || echo no)" '
      [.[] | select(.kind == "stop")] as $s
      | [$h, $r, ($s | length), ([$s[] | select(.hook.blocked == true)] | length),
         ($s | last | .verdict // "-"), $c] | @tsv' "$dir/journal.jsonl"
  done
}

"$@"
