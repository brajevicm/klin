#!/usr/bin/env bash
# Usage: appease.sh SIZES ARNIS_CLONE WORKDIR
# Builds the three appeasement branches of section 3 from arnis highways.rs and
# prints each branch's metrics.
set -eu
sizes=$1 arnis=$2 work=$3
rm -rf "$work" && mkdir -p "$work/src" && cd "$work"
git init -q -b base
git -C "$arnis" show daee5a7bff24:src/element_processing/highways.rs > src/highways.rs
git add -A && git commit -qm base
git checkout -qb split
python3 - <<'PY'
import re
src = open('src/highways.rs').read().split('\n')
cut = next(i for i in range(len(src) // 2, len(src)) if re.match(r'^(pub )?(fn|struct|const|impl|enum) ', src[i]))
open('src/highways.rs', 'w').write('\n'.join(src[:cut]))
open('src/highways2.rs', 'w').write('\n'.join(src[cut:]))
PY
git add -A && git commit -qm split
git checkout -q base && git checkout -qb skipped
mkdir -p src/fixtures && git mv src/highways.rs src/fixtures/highways.rs && git commit -qm skipped
git checkout -q base && git checkout -qb joined
python3 - <<'PY'
src = open('src/highways.rs').read().split('\n')
safe = lambda x: not any(mark in x for mark in ('//', '"', "'", '#', '/*'))
out = []
for line in src:
    text = line.strip()
    if out and text and safe(text) and safe(out[-1]) and len(out[-1]) < 200 and not out[-1].strip().endswith('}'):
        out[-1] = out[-1] + ' ' + text
    else:
        out.append(line)
open('src/highways.rs', 'w').write('\n'.join(out))
PY
git commit -qam joined
for branch in base split skipped joined; do
  echo "== $branch"
  "$sizes" scan . "$branch" | awk -F'\t' 'NR>1{print $1, "lines="$6, "code="$7, "named="$10, "longest="$12, "error="$5}'
done
