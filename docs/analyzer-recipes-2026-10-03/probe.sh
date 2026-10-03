#!/usr/bin/env bash
# probe.sh [FAMILY [ROUTE...]]: lay each route over its base and run klin with
# `{}` and with the recipe entries, at the first Stop and in CI.
# TOOLS names the directory of the pinned tools (see README of section 2).
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
klin=${KLIN:-$repo/target/release/klin}
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
out=${OUT:-$(mktemp -d)}
home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT
mkdir -p "$out"

git_() { git -c user.name=probe -c user.email=probe@example.com -c commit.gpgsign=false "$@"; }

klin_() { env -i PATH="$PATH" HOME="$home" "$klin" "$@"; }

event() {
  printf '{"klin_protocol":1,"event":"%s","root":"%s","session":"probe"%s}' "$1" "$2" "${3:-}"
}

hook() {
  local root=$1 kind=$2 extra=$3
  shift 3
  (cd "$root" && event "$kind" "$root" "$extra" | klin_ "$@")
}

recipes() {
  local entry tool sep=
  printf '{ "sarif": ['
  for tool in "$@"; do
    printf '%s\n  { "name": "%s", "run": "sh %s/recipes/run.sh %s %s %s/recipes", "report": ".klin-recipes/%s.sarif" }' \
      "$sep" "$tool" "$here" "$tool" "$tools" "$here" "$tool"
    sep=,
  done
  printf '\n] }\n'
}

config() {
  case $1:$2 in
    *:none) echo '{}' ;;
    recipes-ts:recipe) recipes eslint semgrep gitleaks ;;
    recipes-py:recipe) recipes ruff semgrep gitleaks ;;
  esac
}

stage() {
  local fixture=$here/fixtures/$1 name=$2 tree=$3 mode=$4
  mkdir -p "$tree"
  cp -R "$fixture/base/." "$tree"
  config "$1" "$mode" > "$tree/klin.json"
  git_ -C "$tree" init -q -b main
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -q -m base
  git_ -C "$tree" checkout -q -b work
  hook "$tree" session "" radius > /dev/null 2>&1 || true
  hook "$tree" prompt ',"prompt":"do the task"' radius > /dev/null 2>&1 || true
  [ "$name" = base ] || cp -R "$fixture/$name/." "$tree"
}

stop() {
  local code=0
  hook "$1" stop ',"blocked_before":false' gate --hook --changed > "$2" 2>&1 || code=$?
  echo "$code"
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
  (cd "$clone" && klin_ gate --strict --json) > "$2" 2> "$2.err" || code=$?
  rm -rf "$(dirname "$clone")"
  echo "$code"
}

# The rules a CI run failed, as gate:rule, from the JSON report.
failed() {
  jq -r '[(.findings[]? | "\(.gate):\(.text | split(": ")[0] | sub("^.*\\.recipes\\."; ""))"),
    (.gates[]? | select(.status == "ERR") | "\(.name):ERR")] | unique | join(" ") | if . == "" then "-" else . end' "$1" 2> /dev/null || echo "?"
}

route() {
  local family=$1 name=$2 mode work stop_code ci_code tag row
  row="$family/$name"
  for mode in none recipe; do
    tag=${family}__${name}__${mode}
    work=$(mktemp -d)
    stage "$family" "$name" "$work/repo" "$mode"
    stop_code=$(stop "$work/repo" "$out/$tag.stop")
    ci_code=$(ci "$work/repo" "$out/$tag.ci.json")
    row="$row	$stop_code	$ci_code"
    [ "$mode" = none ] || row="$row	$(failed "$out/$tag.ci.json")"
    rm -rf "$work"
  done
  printf '%s\n' "$row"
}

family() {
  local family=$1 name
  shift
  if [ $# -gt 0 ]; then
    for name in "$@"; do route "$family" "$name"; done
    return
  fi
  route "$family" base
  while IFS=$'\t' read -r name _ _; do route "$family" "$name"; done < "$here/fixtures/$family/routes.tsv"
}

if [ $# -gt 0 ]; then
  family "$@"
else
  family recipes-ts
  family recipes-py
fi
echo "run text: $out" >&2
