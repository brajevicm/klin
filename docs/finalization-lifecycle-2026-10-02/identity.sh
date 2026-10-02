#!/usr/bin/env bash
# Times the tree-identity mechanisms of the #352 note on a synthetic tree.
# Usage: identity.sh WORKDIR [FILES] [LINES]
set -euo pipefail
work=$1; files=${2:-10000}; lines=${3:-100}
rm -rf "$work"; mkdir -p "$work/tree"; cd "$work/tree"
git init -q; git config user.email p@invalid; git config user.name probe
echo 'node_modules/' > .gitignore
for ((i = 0; i < files; i++)); do
  d=src/m$((i % 100)); mkdir -p "$d"
  awk -v n="$lines" -v i="$i" 'BEGIN { for (l = 0; l < n; l++) printf "export const v%d_%d = %d;\n", i, l, l }' > "$d/f$i.ts"
done
git add -A; git commit -qm base
ms() { local s e; s=$(perl -MTime::HiRes=time -e 'printf "%d", time*1000'); "$@" >/dev/null; e=$(perl -MTime::HiRes=time -e 'printf "%d", time*1000'); echo $((e - s)); }
median() { sort -n | awk '{ a[NR] = $1 } END { print a[int((NR + 1) / 2)] }'; }
fresh() { rm -f "$work/fresh.idx"; GIT_INDEX_FILE="$work/fresh.idx" git add -A; GIT_INDEX_FILE="$work/fresh.idx" git write-tree; }
kept() { GIT_INDEX_FILE="$work/kept.idx" git add -A; GIT_INDEX_FILE="$work/kept.idx" git write-tree; }
row() { local name=$1; shift; local v; v=$(for _ in 1 2 3 4 5; do ms "$@"; done | median); printf '%s\t%s\n' "$name" "$v"; }
kept >/dev/null
echo "files	$files"
row "fresh index: add -A + write-tree, clean" fresh
row "kept index: add -A + write-tree, clean" kept
row "git status --porcelain, clean" git status --porcelain
for ((i = 0; i < 20; i++)); do echo "// edit" >> "src/m$i/f$i.ts"; done
row "fresh index: add -A + write-tree, 20 changed" fresh
kept >/dev/null
row "kept index: add -A + write-tree, 20 changed (warm)" kept
row "git status --porcelain, 20 changed" git status --porcelain
tree=$(kept)
row "snapshot: checkout-index of the tree into a directory" bash -c "rm -rf '$work/snap'; mkdir '$work/snap'; GIT_INDEX_FILE='$work/kept.idx' git checkout-index -a --prefix='$work/snap/'"
row "snapshot: worktree add --detach of a commit of the tree" bash -c "git worktree remove --force '$work/wt' 2>/dev/null || true; c=\$(git commit-tree $tree -p HEAD -m s); git worktree add -q --detach '$work/wt' \$c"
echo "--- an edit and its revert between two identity reads"
before_tree=$(kept); before_stat=$(GIT_INDEX_FILE="$work/kept.idx" git ls-files --debug | shasum)
cp src/m1/f1.ts "$work/keep"; echo "// mid" >> src/m1/f1.ts; sleep 1.1; cp "$work/keep" src/m1/f1.ts
after_tree=$(kept); after_stat=$(GIT_INDEX_FILE="$work/kept.idx" git ls-files --debug | shasum)
echo "tree id equal: $([ "$before_tree" = "$after_tree" ] && echo yes || echo no)"
echo "index stat data equal: $([ "$before_stat" = "$after_stat" ] && echo yes || echo no)"
