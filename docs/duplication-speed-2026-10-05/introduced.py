"""Exact regions that a commit introduces: one side is mostly lines the commit added,
the other side is anywhere in the tree at that commit. Same k, minimizers and T as
calibrate.py, so these pairs are what a turn-time gate would see.

    python3 introduced.py OUT REPO=GIT_DIR:ROOT:LIMIT ...

ROOT is the scanned directory inside the repository ('.' for the whole tree) and
LIMIT the number of newest non-merge commits that touch it. OUT/mapping.json has
the calibrate.py shape plus a commit per match; OUT/corpus/<repo>@<commit>/ holds
the files of every reported span, so holdout.py rules can read them.
"""
import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path, PurePosixPath

from calibrate import test_path
from verify import normalized

K, T, ADDED_SHARE = 41, 60, 0.5
HUNK = re.compile(r'^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@')


def git(repo, *args):
    return subprocess.run(['git', '-C', repo, *args], capture_output=True, check=True).stdout


class Blobs:
    def __init__(self, repo, cache):
        self.repo, self.cache, self.streams = repo, cache, {}
        cache.mkdir(parents=True, exist_ok=True)

    def text(self, sha):
        return git(self.repo, 'cat-file', 'blob', sha).decode(errors='replace')

    def stream(self, sha, suffix):
        if sha not in self.streams:
            path = self.cache / f'{sha}{suffix}'
            if not path.exists():
                path.write_bytes(git(self.repo, 'cat-file', 'blob', sha))
            n = normalized(path)
            n['tokens'] = [sys.intern(t) if safe else None for t, safe in zip(n['tokens'], n['safe'])]
            self.streams[sha] = n
        return self.streams[sha]


def eligible(root, path):
    relative = PurePosixPath(path).relative_to(root) if root != '.' else PurePosixPath(path)
    return (relative.suffix in ('.rs', '.ts', '.tsx') and not relative.name.endswith('.d.ts')
            and not test_path(relative)), str(relative)


def tree(repo, commit, root):
    files = {}
    for line in git(repo, 'ls-tree', '-r', commit, '--', root).decode().splitlines():
        meta, path = line.split('\t', 1)
        ok, relative = eligible(root, path)
        if ok and meta.split()[1] == 'blob':
            files[relative] = meta.split()[2]
    return files


def added_lines(repo, commit, root):
    added, current = defaultdict(set), None
    diff = git(repo, 'diff', '-U0', '-M', '--no-color', f'{commit}^', commit, '--', root).decode(errors='replace')
    for line in diff.splitlines():
        if line.startswith('+++ '):
            current = None if line == '+++ /dev/null' else line[6:]
        elif current and (m := HUNK.match(line)):
            start, count = int(m[1]), int(m[2] or 1)
            added[current].update(range(start, start + count))
    return added


def scan(name, repo, root, limit, out, blobs):
    commits = git(repo, 'rev-list', '--no-merges', f'--max-count={limit}', 'HEAD', '--', root).decode().split()
    regions, seen = [], set()
    for commit in reversed(commits):
        try:
            added = added_lines(repo, commit, root)
        except subprocess.CalledProcessError:
            continue
        files = tree(repo, commit, root)
        changed = {}
        for path, lines in added.items():
            ok, relative = eligible(root, path)
            if ok and relative in files:
                changed[relative] = lines
        if not changed:
            continue
        paths = sorted(files)
        streams = [blobs.stream(files[p], PurePosixPath(p).suffix) for p in paths]
        postings = defaultdict(list)
        for f, s in enumerate(streams):
            a = s['tokens']
            for position in s['positions']:
                key = tuple(a[position:position + K])
                if len(key) == K and None not in key:
                    postings[key].append((f, position))
        index = {p: i for i, p in enumerate(paths)}
        for path, lines in changed.items():
            file = index[path]
            s = streams[file]
            a, rows = s['tokens'], s['rows']
            coverage = {}
            for position in s['positions']:
                if rows[position] + 1 not in lines:
                    continue
                key = tuple(a[position:position + K])
                if len(key) != K or None in key:
                    continue
                for other, start in postings[key]:
                    if (other, start) == (file, position):
                        continue
                    diagonal = start - position
                    prior = coverage.get((other, diagonal))
                    if prior and prior[0] <= position < prior[1]:
                        continue
                    b = streams[other]['tokens']
                    left, right = position, position + K
                    while left > 0 and left + diagonal > 0 and a[left-1] is not None and a[left-1] == b[left+diagonal-1]:
                        left -= 1
                    while right < len(a) and right + diagonal < len(b) and a[right] is not None and a[right] == b[right+diagonal]:
                        right += 1
                    coverage[other, diagonal] = (left, right)
                    if right - left < T:
                        continue
                    new_rows = {rows[i] + 1 for i in range(left, right)}
                    if len(new_rows & lines) < ADDED_SHARE * len(new_rows):
                        continue
                    content = '\n'.join(a[left:right])
                    family = hashlib.sha256(content.encode()).hexdigest()
                    other_rows = streams[other]['rows']
                    sides = [(path, rows, left), (paths[other], other_rows, left + diagonal)]
                    spans = []
                    for p, r, offset in sides:
                        spans.append({'repo': f'{name}@{commit[:10]}', 'path': p, 'language': 'Rust' if p.endswith('.rs') else 'TypeScript',
                                      'start_line': r[offset] + 1, 'end_line': r[offset + right - left - 1] + 1})
                    identity = (family, spans[0]['path'], spans[1]['path'])
                    if identity in seen:
                        continue
                    seen.add(identity)
                    for span in spans:
                        target = out / 'corpus' / span['repo'] / span['path']
                        if not target.exists():
                            target.parent.mkdir(parents=True, exist_ok=True)
                            target.write_text(blobs.text(files[span['path']]))
                        text = target.read_text().splitlines()
                        span['context'] = '\n'.join(text[max(0, span['start_line'] - 4):span['end_line'] + 3])
                    pair_id = hashlib.sha256(json.dumps([{k: v for k, v in s.items() if k != 'context'} for s in spans], sort_keys=True).encode()).hexdigest()[:16]
                    regions.append({'id': pair_id, 'tokens': right - left, 'language': spans[0]['language'], 'family': family,
                                    'commit': commit, 'spans': spans})
        print(name, commit[:10], len(regions), flush=True)
    return regions


def main():
    out = Path(sys.argv[1])
    regions, counts = [], {}
    for spec in sys.argv[2:]:
        name, rest = spec.split('=', 1)
        repo, root, limit = rest.rsplit(':', 2)
        found = scan(name, repo, root, int(limit), out, Blobs(repo, out / 'blobs' / name))
        counts[name] = {'commit_limit': int(limit), 'root': root, 'pairs': len(found)}
        regions += found
    regions.sort(key=lambda r: r['id'])
    blind = [{'id': r['id'], 'spans': r['spans']} for r in regions]
    for r in regions:
        for s in r['spans']:
            s.pop('context')
    (out / 'mapping.json').write_text(json.dumps({'counts': counts, 'matches': regions}, indent=2) + '\n')
    (out / 'blind.jsonl').write_text(''.join(json.dumps(b) + '\n' for b in blind))
    print(counts)


if __name__ == '__main__':
    main()
