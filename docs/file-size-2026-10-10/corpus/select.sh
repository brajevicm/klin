#!/usr/bin/env bash
# Usage: corpus/select.sh SIZES CLONES
# Walks each saved search in order, clones at depth 1, applies the eligibility
# rules of the note, and writes corpus/selection.json and corpus/skipped.tsv.
set -u
sizes=$1 clones=$2 here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$clones"
: > "$here/skipped.tsv"
echo '[]' > "$here/selection.json"
for lang in rust typescript; do
  manifest=Cargo.toml; key=rust
  [ "$lang" = typescript ] && { manifest=package.json; key=ts; }
  taken=0
  for repo in $(jq -r '.items[].full_name' "$here/search-$lang.json"); do
    [ "$taken" -ge 50 ] && break
    dir="$clones/${repo/\//__}"
    if [ ! -d "$dir/.git" ]; then
      rm -rf "$dir"
      if ! perl -e 'alarm shift; exec @ARGV' 600 git clone -q --depth 1 "https://github.com/$repo.git" "$dir" 2>/dev/null; then
        printf '%s\t%s\tclone failed\n' "$lang" "$repo" >> "$here/skipped.tsv"; continue
      fi
    fi
    if ! git -C "$dir" cat-file -e "HEAD:$manifest" 2>/dev/null; then
      printf '%s\t%s\tno %s at the root\n' "$lang" "$repo" "$manifest" >> "$here/skipped.tsv"; continue
    fi
    files=$("$sizes" scan "$dir" HEAD | awk -F'\t' -v k="$key" 'NR>1 && $2==k && $3==0 && $4==0' | wc -l | tr -d ' ')
    if [ "$files" -lt 50 ]; then
      printf '%s\t%s\t%s production files of the language\n' "$lang" "$repo" "$files" >> "$here/skipped.tsv"; continue
    fi
    commit=$(git -C "$dir" rev-parse HEAD)
    jq --arg l "$lang" --arg r "$repo" --arg c "$commit" --argjson f "$files" \
      '. + [{language: $l, repository: $r, commit: $c, production_files: $f}]' \
      "$here/selection.json" > "$here/selection.tmp" && mv "$here/selection.tmp" "$here/selection.json"
    taken=$((taken + 1))
  done
done
