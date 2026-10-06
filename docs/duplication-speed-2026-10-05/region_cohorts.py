"""Exact T-token seed diagnostic; exhaustive oracle remains independent."""
import itertools
import json
import random
import resource
import runpy
import subprocess
import sys
import time
from pathlib import Path

import families


def indexed(records, threshold):
    assert threshold > 0
    groups = {}
    for name, stream in records:
        key = (tuple(stream['tokens']), tuple(stream['safe']))
        groups.setdefault(key, []).append(name)
    classes = list(groups.items())
    postings = {}
    output = {}
    grams = 0
    for class_id, ((tokens, safe), names) in enumerate(classes):
        first = 0
        for end in range(len(tokens) + 1):
            if end < len(tokens) and safe[end]:
                continue
            if len(names) > 1 and end - first >= threshold:
                families.add(output, tokens, names, names, first, first, end)
            for start in range(first, end - threshold + 1):
                postings.setdefault(tokens[start:start + threshold], []).append((class_id, start))
                grams += 1
            first = end + 1
    candidates = starts = extended_tokens = 0
    # ponytail: exact tuple keys and pair enumeration; repeated seeds still cost
    # O(multiplicity²). A compressed suffix/cohort algorithm is the upgrade.
    for cohort in postings.values():
        for (left_id, a), (right_id, b) in itertools.combinations(cohort, 2):
            candidates += 1
            (left, left_safe), left_names = classes[left_id]
            (right, right_safe), right_names = classes[right_id]
            if a and b and left_safe[a - 1] and right_safe[b - 1] and left[a - 1] == right[b - 1]:
                continue
            starts += 1
            end, other_end = a + threshold, b + threshold
            while end < len(left) and other_end < len(right) and left_safe[end] and right_safe[other_end] and left[end] == right[other_end]:
                end += 1
                other_end += 1
                extended_tokens += 1
            families.add(output, left, left_names, right_names, a, b, end)
    return output, {'classes': len(classes), 'grams': grams, 'seed_keys': len(postings), 'seed_pairs': candidates, 'maximal_starts': starts, 'extended_tokens': extended_tokens, 'families': len(output), 'occurrences': sum(map(len, output.values()))}


def stream(tokens, safe=None):
    return {'tokens': tokens, 'safe': [True] * len(tokens) if safe is None else safe}


def cases():
    interior = list(range(30))
    yield 'arbitrary-interior', [(f'f{i}', stream([100 + i] + interior + [200 + i])) for i in range(12)], 12
    yield 'self-overlap', [('a', stream(['x'] * 18))], 4
    yield 'XY-YZ', [('a', stream(list('xxxxyyyy'))), ('b', stream(list('xxxxyyyyzzzz'))), ('c', stream(list('yyyyzzzz')))], 4
    yield 'unsafe-boundary', [('a', stream(interior, [i != 15 for i in range(30)])), ('b', stream(interior))], 8
    yield 'language-provenance', [('a', stream(['ts:x'] * 10)), ('b', stream(['rs:x'] * 10)), ('c', stream(['ts:other-import:x'] * 10))], 4
    yield '1000-identical', [(str(i), stream(interior)) for i in range(1000)], 12
    yield '64-short-decoys', [('a', stream(interior)), *[(f'd{i}', stream(interior[:11] + [100 + i])) for i in range(64)], ('z', stream(interior))], 12
    for seed in range(240):
        rng = random.Random(seed)
        records = [(str(i), stream([rng.randrange(2 if seed % 2 else 12) for _ in range(rng.randrange(0, 35))])) for i in range(rng.randrange(1, 7))]
        for _, item in records:
            item['safe'] = [rng.random() > .08 for _ in item['tokens']]
        yield f'random-{seed}', records, rng.randrange(1, 10)


def corpus(root):
    # Reuse the identical eligibility predicate used by the encoding experiment.
    eligible = runpy.run_path(str(Path(__file__).parent / 'results-optimizations/compression.py'))['eligible']
    started = time.perf_counter()
    records = []
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if path.is_file() and eligible(relative):
            data = json.loads(subprocess.check_output([str(families.verify.BIN), 'normalize', str(relative)], cwd=root))
            records.append((str(relative), data))
    normalization_ms = (time.perf_counter() - started) * 1000
    started = time.perf_counter()
    _, metrics = indexed(records, 60)
    indexed_ms = (time.perf_counter() - started) * 1000
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    print(json.dumps({'scope': 'pinned corpus generic interior diagnostic; no exhaustive corpus oracle or production budget claim', 'root': str(root), 'threshold': 60, 'records': len(records), 'tokens': sum(len(s['tokens']) for _, s in records), 'normalization_ms': normalization_ms, 'index_and_report_ms': indexed_ms, 'process_peak_rss_bytes': peak if sys.platform == 'darwin' else peak * 1024, 'rss_scope': 'parent Python process lifetime; excludes normalizer subprocess RSS', **metrics}, indent=2))


def main():
    if len(sys.argv) > 1:
        corpus(Path(sys.argv[1]).resolve())
        return
    rows = []
    for label, records, threshold in cases():
        actual, metrics = indexed(records, threshold)
        # Avoid expanding the deliberately large identical-class case's oracle.
        if label == '1000-identical':
            assert actual == {tuple(range(30)): {(str(i), 0, 30) for i in range(1000)}}
        else:
            expected = families.exhaustive(records, threshold)
            assert actual == expected, label
        diagonals = sum(max(0, len(a['tokens']) - 1) if i == j else max(0, len(a['tokens']) + len(b['tokens']) - 1) for i, (_, a) in enumerate(records) for j, (_, b) in enumerate(records) if j >= i)
        rows.append({'case': label, 'threshold': threshold, 'records': len(records), 'verification': 'exact-expected-mapping' if label == '1000-identical' else 'exhaustive-oracle', 'oracle_equal': None if label == '1000-identical' else True, 'exhaustive_candidate_diagonals': diagonals, **metrics})
    print(json.dumps({'scope': 'exact current-content region families; normalized tokens only; no lineage or cache qualification', 'cases': rows}, indent=2))


if __name__ == '__main__':
    main()
