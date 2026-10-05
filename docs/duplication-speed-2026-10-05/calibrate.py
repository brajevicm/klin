"""Blind check-only calibration packets; exact text verifies every selected seed.

All production files in the three pinned roots, not just changed-file samples.
Candidate enumeration uses the frozen minimizers; it is not the independent oracle.
"""
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path
from verify import normalized


ROOTS = [('klin', 'klin/src'), ('glaredb', 'glaredb'), ('karakeep', 'karakeep')]


def collect(corpus, output, roots=ROOTS):
    streams, origins, texts, counts = [], [], [], {}
    for repo, root in [(repo, corpus / relative) for repo, relative in roots]:
        count = {'files': 0, 'tokens': 0, 'unsafe_units': 0}
        for path in sorted(root.rglob('*')):
            relative = path.relative_to(root)
            if not path.is_file() or path.suffix not in ('.rs', '.ts', '.tsx') or path.name.endswith(('.d.ts', '_test.rs')) or path.name == 'tests.rs':
                continue
            if any(part in ('tests', 'test', '__tests__', 'benches', 'fixtures') for part in relative.parts) or '.test.' in path.name or '.spec.' in path.name:
                continue
            n = normalized(path)
            count['files'] += 1
            count['tokens'] += len(n['tokens'])
            count['unsafe_units'] += n['unsafe_units']
            # Unsafe tokens are boundaries, never evidence.
            n['tokens'] = [sys.intern(token) if safe else None for token, safe in zip(n['tokens'], n['safe'])]
            streams.append(n)
            origins.append({'repo': repo, 'path': str(relative), 'language': 'Rust' if path.suffix == '.rs' else 'TypeScript'})
            texts.append(path.read_text().splitlines())
        counts[repo] = count
        print(repo, count, flush=True)
    postings, coverage, regions = defaultdict(list), {}, []
    for file, stream in enumerate(streams):
        a = stream['tokens']
        for position in stream['positions']:
            key = tuple(a[position:position + 41])
            if len(key) != 41 or None in key:
                continue
            for other, start in postings[key]:
                diagonal = start - position
                prior = coverage.get((file, other, diagonal))
                if prior and prior[0] <= position < prior[1]:
                    continue
                b = streams[other]['tokens']
                left, right = position, position + 41
                while left > 0 and left + diagonal > 0 and a[left-1] is not None and a[left-1] == b[left+diagonal-1]:
                    left -= 1
                while right < len(a) and right + diagonal < len(b) and a[right] is not None and a[right] == b[right+diagonal]:
                    right += 1
                coverage[file, other, diagonal] = (left, right)
                if right - left < 60:
                    continue
                spans = []
                for id, offset in [(file, left), (other, left + diagonal)]:
                    rows = streams[id]['rows']
                    first, last = rows[offset], rows[offset + right-left-1]
                    spans.append({**origins[id], 'start_line': first+1, 'end_line': last+1,
                                  'context': '\n'.join(texts[id][max(0, first-3):last+4])})
                content = '\n'.join(a[left:right])
                identity = hashlib.sha256(json.dumps(spans, sort_keys=True).encode()).hexdigest()[:16]
                regions.append({'id': identity, 'tokens': right-left, 'language': origins[file]['language'],
                                'family': hashlib.sha256(content.encode()).hexdigest(), 'spans': spans})
            postings[key].append((file, position))
    output.mkdir(exist_ok=True)
    regions.sort(key=lambda r: r['id'])
    (output / 'mapping.json').write_text(json.dumps({'counts': counts, 'matches': regions}, indent=2) + '\n')
    (output / 'blind.jsonl').write_text(''.join(json.dumps({'id': r['id'], 'spans': r['spans']}) + '\n' for r in regions))
    print(len(regions), 'matches; labels must be reviewed before selection', flush=True)


def summarize(output):
    data = json.loads((output / 'mapping.json').read_text())
    labels = json.loads((output / 'labels.json').read_text())
    assert set(labels) == {r['id'] for r in data['matches']}, 'every eligible match must be labeled'
    result = {}
    for language in ('Rust', 'TypeScript'):
        rows = [r for r in data['matches'] if r['language'] == language]
        def at(t):
            matches = [r for r in rows if r['tokens'] >= t]
            return {'threshold': t, 'non_copy': sum(labels[r['id']]['label'] != 'copy' for r in matches),
                    'total': len(matches), 'repositories': sorted({s['repo'] for r in matches for s in r['spans']})}
        candidates = [at(t) for t in (60, 80, 100, 150)]
        selected = next((r['threshold'] for r in candidates if r['total'] and not r['non_copy']), None)
        result[language] = {'candidates': candidates, 'selected': selected,
                            'sensitivity': [at(selected+d) for d in (-20, -10, 10, 20)] if selected else []}
    (output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    if sys.argv[1] == 'collect':
        roots = [tuple(pair.split('=', 1)) for pair in sys.argv[4:]] or ROOTS
        collect(Path(sys.argv[2]), Path(sys.argv[3]), roots)
    else:
        summarize(Path(sys.argv[2]))
