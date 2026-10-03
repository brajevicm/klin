#!/usr/bin/env bash
# contract.sh: what the Stop and CI print when a recipe's tool is missing,
# fails, writes a bad report, runs past the limit, or reads less than it was
# given. Each case is one `sarif` entry over a fixture route.
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
klin_() { env -i PATH="$PATH" HOME="$home" ${LIMIT:+KLIN_COMMAND_LIMIT=$LIMIT} "$klin" "$@"; }
event() { printf '{"klin_protocol":1,"event":"%s","root":"%s","session":"probe"%s}' "$1" "$2" "${3:-}"; }
hook() {
  local root=$1 kind=$2 extra=$3
  shift 3
  (cd "$root" && event "$kind" "$root" "$extra" | klin_ "$@")
}

recipe() { echo "sh $here/recipes/run.sh $1 $tools $here/recipes; mv .klin-recipes/$1.sarif .klin-recipes/recipe.sarif"; }

case_() {
  local name=$1 family=$2 route=$3 run=$4 extra=${5:-} tree code ci clone
  tree=$(mktemp -d)/repo
  mkdir -p "$tree"
  cp -R "$here/fixtures/$family/base/." "$tree"
  jq -n --arg run "$run" '{sarif: [{name: "recipe", run: $run, report: ".klin-recipes/recipe.sarif"}]}' > "$tree/klin.json"
  git_ -C "$tree" init -q -b main
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -q -m base
  git_ -C "$tree" checkout -q -b work
  hook "$tree" session "" radius > /dev/null 2>&1 || true
  hook "$tree" prompt ',"prompt":"do the task"' radius > /dev/null 2>&1 || true
  [ "$route" = base ] || cp -R "$here/fixtures/$family/$route/." "$tree"
  [ -z "$extra" ] || (cd "$tree" && eval "$extra")
  code=0
  hook "$tree" stop ',"blocked_before":false' gate --hook --changed > "$out/$name.stop" 2>&1 || code=$?
  git_ -C "$tree" add -A
  git_ -C "$tree" commit -q -m route
  clone=$(mktemp -d)/ci
  git_ -C "$tree" checkout -q main
  git_ clone -q "$tree" "$clone" 2> /dev/null
  git_ -C "$clone" checkout -q work
  ci=0
  (cd "$clone" && klin_ gate --strict) > "$out/$name.ci" 2>&1 || ci=$?
  printf '%s\t%s\t%s\t%s\n' "$name" "$code" "$ci" "$(grep -E '^ *(ok|FAIL|ERR) +recipe' "$out/$name.ci" | awk '{print $1}')"
  rm -rf "$(dirname "$tree")" "$(dirname "$clone")"
}

mkreport() { printf "mkdir -p .klin-recipes && printf '%s' > .klin-recipes/recipe.sarif" "$1"; }

case_ present-ruff recipes-py debug-plant-print "$(recipe ruff)"
case_ missing-tool recipes-py debug-plant-print "mkdir -p .klin-recipes && /nonexistent/ruff check -o .klin-recipes/recipe.sarif ."
case_ crash-no-report recipes-py debug-plant-print "echo boom >&2; exit 3"
case_ truncated-report recipes-py debug-plant-print "$(mkreport '{"version":"2.1.0","runs":[{"results":[')"
case_ empty-runs recipes-py debug-plant-print "$(mkreport '{"version":"2.1.0","runs":[]}')"
case_ failed-invocation recipes-py debug-plant-print "$(mkreport '{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"x"}},"invocations":[{"executionSuccessful":false}],"results":[]}]}')"
LIMIT=2 case_ past-limit recipes-py debug-plant-print "sleep 5; $(recipe ruff)"
case_ eslint-present recipes-ts debug-plant-log "$(recipe eslint)"
case_ eslint-parse-error recipes-ts base "$(recipe eslint)" "printf 'console.log(1);\nexport const broken = (;\n' > src/broken.ts"
case_ eslint-outside-project recipes-ts base "$(recipe eslint)" "mkdir -p tools && printf 'console.log(1);\nexport {};\n' > tools/dump.ts"
case_ ruff-parse-error recipes-py base "$(recipe ruff)" "printf 'print(1)\ndef f(:\n' > shop/broken.py"
case_ semgrep-parse-error recipes-ts inj-plant-sql "$(recipe semgrep)" "printf 'export const broken = (;\n' >> src/users.ts"
echo "run text: $out" >&2
