#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
clones=${1:?usage: replay.sh CLONES}
out=${2:-$here}
shapes_target=${SHAPES_TARGET:-$(mktemp -d)}
CARGO_TARGET_DIR=$shapes_target cargo build --quiet --release --manifest-path "$here/../prototype/Cargo.toml"
shapes=$shapes_target/release/shapes

export_tree() {
  rm -rf "$3"
  mkdir -p "$3"
  git -C "$1" archive "$2" | tar -x -C "$3"
}

changes() {
  jq -r '.repositories[] | .fullName as $name | .start as $start | .changes[] | "\($name) \($start) \(.base) \(.head)"' \
    "$repo/benchmark/evidence/false-alarms-2026-09-29/selection.json" "$here/selection-python.json"
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: > "$out/new-sites.tsv"
: > "$out/start-sites.tsv"
: > "$out/changes.tsv"
last=
while read -r name start base head; do
  dir=$clones/${name//\//__}
  if [ "$name" != "$last" ]; then
    export_tree "$dir" "$start" "$work/start"
    "$shapes" scan "$work/start" | sed "s|^|$name\t|" >> "$out/start-sites.tsv"
    last=$name
  fi
  files=()
  while read -r file; do files+=("$file"); done < <(git -C "$dir" diff --name-only --diff-filter=AMR "$base" "$head" | grep -E '\.(rs|py|pyi|ts|mts|cts|tsx)$' || true)
  printf '%s\t%s\t%s\n' "$name" "$head" "${#files[@]}" >> "$out/changes.tsv"
  [ ${#files[@]} -gt 0 ] || continue
  export_tree "$dir" "$base" "$work/base"
  export_tree "$dir" "$head" "$work/head"
  "$shapes" new "$work/base" "$work/head" "${files[@]}" | sed "s|^|$name\t${head:0:10}\t|" >> "$out/new-sites.tsv"
done < <(changes)
awk -F'\t' '{ n = split($4, part, "."); ext = part[n]; lang = ext == "rs" ? "rust" : (ext == "py" || ext == "pyi") ? "python" : "ts"; key = $2 "\t" lang; all[key]++; if ($6 == "-") found[key]++ }
  END { for (key in all) print key "\t" all[key] "\t" found[key] + 0 }' "$out/start-sites.tsv" | sort > "$out/start-sites-summary.tsv"
wc -l "$out/changes.tsv" "$out/new-sites.tsv" "$out/start-sites.tsv"
