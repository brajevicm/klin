#!/usr/bin/env bash
# usage: KLIN_BIN=PATH replay.sh CLONES
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
clones=${1:?usage: KLIN_BIN=PATH replay.sh CLONES}
klin=${KLIN_BIN:?KLIN_BIN names the klin binary}
deps=$repo/docs/dependency-evidence-2026-10-02/prototype/deps.py
shapes_target=${SHAPES_TARGET:-$(mktemp -d)}
CARGO_TARGET_DIR=$shapes_target cargo build --quiet --release --manifest-path "$repo/docs/unfinished-code-2026-10-02/prototype/Cargo.toml"
shapes=$shapes_target/release/shapes
cp "$klin.provenance" "$here/klin.provenance.json"

export_tree() {
  rm -rf "$3"
  mkdir -p "$3"
  git -C "$1" archive "$2" | tar -x -C "$3"
}

prepare() {
  git -C "$1" remote remove origin 2> /dev/null || true
  git -C "$1" checkout --quiet --force --detach "$3"
  git -C "$1" for-each-ref --format='%(refname:short)' refs/heads | while read -r branch; do
    git -C "$1" branch --quiet -D "$branch"
  done
  git -C "$1" branch --quiet main "$2"
  git -C "$1" checkout --quiet -b change
  git -C "$1" reset --quiet --hard
  git -C "$1" clean --quiet -ffdx
  rm -rf "$1/.git/klin"
  printf '{}\n' > "$1/klin.json"
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$here/runs"
: > "$here/shapes.tsv"
: > "$here/deps.tsv"
: > "$here/added.tsv"
: > "$here/tests.tsv"
while read -r arm name number base head; do
  dir=$clones/${name//\//__}
  key=${name//\//__}-$number
  files=()
  while read -r file; do files+=("$file"); done < <(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head" | grep -E '\.(rs|py|pyi|ts|mts|cts|tsx)$' || true)
  export_tree "$dir" "$base" "$work/base"
  export_tree "$dir" "$head" "$work/head"
  if [ ${#files[@]} -gt 0 ]; then
    "$shapes" new "$work/base" "$work/head" "${files[@]}" | sed "s|^|$key\t|" >> "$here/shapes.tsv"
  fi
  python3 "$deps" new "$work/base" "$work/head" | sed "s|^|$key\t|" >> "$here/deps.tsv"
  python3 "$deps" added "$work/base" "$work/head" | sed "s|^|$key\t|" >> "$here/added.tsv"
  python3 "$here/tests.py" "$dir" "$base" "$head" | sed "s|^|$key\t|" >> "$here/tests.tsv"
  out=$here/runs/$key.json
  if [ ! -f "$out" ]; then
    prepare "$dir" "$base" "$head"
    started=$(python3 -c 'import time; print(time.time())')
    set +e
    report=$(env -u GITHUB_BASE_REF -u GITHUB_EVENT_PATH $(env | sed -n 's/^\(KLIN_[A-Z_]*\)=.*/-u \1/p') \
      perl -e 'alarm 600; exec @ARGV' "$klin" gate --json 2> "$work/stderr")
    status=$?
    set -e
    ms=$(python3 -c "import time; print(round((time.time() - $started) * 1000))")
    jq -n --arg arm "$arm" --arg name "$name" --argjson number "$number" --arg base "$base" --arg head "$head" \
      --argjson exit "$status" --argjson ms "$ms" --arg stderr "$(cat "$work/stderr")" --arg raw "$report" \
      '{arm: $arm, repository: $name, number: $number, base: $base, head: $head, exit: $exit, ms: $ms,
        report: ($raw | try fromjson catch null), stdout: ($raw | try (fromjson | null) catch $raw), stderr: $stderr}' > "$out"
    echo "$key exit $status in $ms ms"
  fi
done < <(jq -r '(.agent[] | "agent \(.fullName) \(.number) \(.base) \(.head)"), (.human[] | "human \(.fullName) \(.number) \(.base) \(.head)")' "$here/selection.json")
wc -l "$here/shapes.tsv" "$here/deps.tsv" "$here/added.tsv" "$here/tests.tsv"
