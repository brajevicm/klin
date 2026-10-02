#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
mkdir -p "$1"; work=$(cd "$1" && pwd); files=${2:-10000}; lines=${3:-100}
rm -rf "$work/tree" "$work/copy" "$work/wt"; mkdir -p "$work/tree"; cd "$work/tree"
git_() { git -c user.email=p@invalid -c user.name=probe -c commit.gpgsign=false "$@"; }
git_ init -q
echo 'node_modules/' > .gitignore
for ((i = 0; i < files; i++)); do
  d=src/m$((i % 100)); mkdir -p "$d"
  awk -v n="$lines" -v i="$i" 'BEGIN { for (l = 0; l < n; l++) printf "export const v%d_%d = %d;\n", i, l, l }' > "$d/f$i.ts"
done
git_ add -A; git_ commit -qm base
now() { perl -MTime::HiRes=time -e 'printf "%d", time*1000'; }
median() { sort -n | awk '{ a[NR] = $1 } END { print a[int((NR + 1) / 2)] }'; }
tree_of() { GIT_INDEX_FILE=$1 git add -A; GIT_INDEX_FILE=$1 git write-tree; }
fresh() { rm -f "$work/fresh.idx"; tree_of "$work/fresh.idx"; }
kept() { tree_of "$work/kept.idx"; }
copy() { mkdir "$work/copy"; GIT_INDEX_FILE="$work/kept.idx" git checkout-index -a --prefix="$work/copy/"; }
uncopy() { rm -rf "$work/copy"; }
worktree() { git_ worktree add -q --detach "$work/wt" "$(git_ commit-tree "$(kept)" -p HEAD -m copy)"; }
unworktree() { git_ worktree remove --force "$work/wt"; }
nothing() { :; }
status() { git status --porcelain; }
row() {
  local name=$1 timed=$2 after=${3:-nothing} s e
  for _ in 1 2 3 4 5; do
    s=$(now); "$timed" >/dev/null; e=$(now); "$after"; echo $((e - s))
  done | median | sed "s/^/$name	/"
}
kept >/dev/null
echo "files	$files"
row "fresh index: add -A + write-tree, clean" fresh
row "kept index: add -A + write-tree, clean" kept
row "git status --porcelain, clean" status
for ((i = 0; i < 20; i++)); do echo "// edit" >> "src/m$i/f$i.ts"; done
row "fresh index: add -A + write-tree, 20 changed" fresh
kept >/dev/null
row "kept index: add -A + write-tree, 20 changed (warm)" kept
row "git status --porcelain, 20 changed" status
row "immutable copy: checkout-index of the tree into a directory" copy uncopy
row "immutable copy: worktree add --detach of a commit of the tree" worktree unworktree
echo "--- an edit and its revert between two identity reads"
before_tree=$(kept); before_stat=$(GIT_INDEX_FILE="$work/kept.idx" git ls-files --debug | shasum)
cp src/m1/f1.ts "$work/keep"; echo "// mid" >> src/m1/f1.ts; sleep 1.1; cp "$work/keep" src/m1/f1.ts
after_tree=$(kept); after_stat=$(GIT_INDEX_FILE="$work/kept.idx" git ls-files --debug | shasum)
echo "tree id equal: $([ "$before_tree" = "$after_tree" ] && echo yes || echo no)"
echo "index stat data equal: $([ "$before_stat" = "$after_stat" ] && echo yes || echo no)"
