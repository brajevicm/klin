#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
klin=${KLIN:-$repo/target/release/klin}
out=${OUT:-$(mktemp -d)}
home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT
tools=(RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}")
mkdir -p "$out"
deps=$here/prototype/deps.py

git_() { git -c user.name=probe -c user.email=probe@example.com -c commit.gpgsign=false "$@"; }

klin_() { env -i PATH="$PATH" HOME="$home" "${tools[@]}" "$klin" "$@"; }

event() {
  printf '{"klin_protocol":1,"event":"%s","root":"%s","session":"probe"%s}' "$1" "$2" "${3:-}"
}

hook() {
  local root=$1 kind=$2 extra=$3
  shift 3
  (cd "$root" && event "$kind" "$root" "$extra" | klin_ "$@")
}

verdict() { tail -n 1 "$1/.git/klin/journal.jsonl" | jq -r '.verdict // "none"'; }

stage() {
  local fixture=$here/fixtures/$1 name=$2 tree=$3
  mkdir -p "$tree"
  cp -R "$fixture/base/." "$tree"
  [ -f "$tree/klin.json" ] || echo '{}' > "$tree/klin.json"
  git_ -C "$tree" init -q -b main
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -q -m base
  git_ -C "$tree" checkout -q -b work

  hook "$tree" session "" radius > /dev/null 2>&1 || true
  hook "$tree" prompt ',"prompt":"do the task"' radius > /dev/null 2>&1 || true

  if [ "$name" != base ]; then
    cp -R "$fixture/$name/." "$tree"
    if [ -f "$fixture/$name.delete" ]; then
      (cd "$tree" && xargs rm -rf < "$fixture/$name.delete")
    fi
  fi
  find "$tree" -name '*.sarif' -not -path '*/.git/*' -exec touch {} +
}

stop() {
  local code=0
  hook "$1" stop ",\"blocked_before\":$2" gate --hook --changed > "$3" 2>&1 || code=$?
  echo "$code $(verdict "$1")"
}

ci() {
  local tree=$1 clone code=0
  clone=$(mktemp -d)/ci
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -q --allow-empty -m route
  git_ -C "$tree" checkout -q main
  git_ clone -q "$tree" "$clone" 2> /dev/null
  git_ -C "$tree" checkout -q work
  git_ -C "$clone" checkout -q work
  (cd "$clone" && klin_ gate --strict) > "$2" 2>&1 || code=$?
  rm -rf "$(dirname "$clone")"
  echo "$code"
}

route() {
  local family=$1 name=$2 work first second before=false
  local tag=${family}__${name}
  work=$(mktemp -d)
  stage "$family" "$name" "$work/repo"
  first=$(stop "$work/repo" false "$out/$tag.stop1")
  [ "${first%% *}" = 2 ] && before=true
  second=$(stop "$work/repo" "$before" "$out/$tag.stop2")
  printf '%s/%s\t%s\t%s\t%s\t%s\n' "$family" "$name" "$first" "$second" "$(ci "$work/repo" "$out/$tag.ci")" "$(found "$family" "$name" "$out/$tag.deps")"
  rm -rf "$work"
}

found() {
  local fixture=$here/fixtures/$1 work
  work=$(mktemp -d)
  mkdir -p "$work/before" "$work/after"
  cp -R "$fixture/base/." "$work/before"
  cp -R "$fixture/base/." "$work/after"
  [ "$2" = base ] || cp -R "$fixture/$2/." "$work/after"
  python3 "$deps" probe "$work/before" "$work/after" | tee "$3"
  rm -rf "$work"
}

family() {
  local family=$1 dir
  route "$family" base
  for dir in "$here/fixtures/$family"/*/; do
    dir=$(basename "$dir")
    [ "$dir" = base ] || route "$family" "$dir"
  done
}

all() {
  local dir
  for dir in "$here/fixtures"/*/; do
    family "$(basename "$dir")"
  done
}

if [ "${1:-}" = stage ]; then
  stage "$2" "$3" "$4"
  first=$(stop "$4" false "$4.stop1")
  [ "${first%% *}" = 2 ] && touch "$4.blocked"
  echo "$first"
elif [ "${1:-}" = finish ]; then
  before=false
  [ -f "$2.blocked" ] && before=true
  next=$(stop "$2" "$before" "$2.stop2")
  [ "${next%% *}" = 2 ] && touch "$2.blocked"
  echo "stop: $next"
  echo "ci: $(ci "$2" "$2.ci")"
elif [ "${1:-}" = --check ]; then
  diff "$here/expected.tsv" <(all) && echo "every route matches expected.tsv"
elif [ $# -gt 0 ]; then
  for name in "$@"; do family "$name"; done
else
  all
fi
echo "run text: $out" >&2
