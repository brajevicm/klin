"""Diagnostic compressed exact-region output; not a lineage implementation."""
import json
import os
import sys
import tempfile
import time
from pathlib import Path

import verify

verify.BIN = Path(os.environ.get('DUP_SPEED', '/tmp/klin490-opt-reference/dup-speed'))


def add(families, tokens, left, right, a, b, end):
    key = tuple(tokens[a:end])
    families.setdefault(key, set()).update((name, a, end) for name in left)
    families[key].update((name, b, b + end - a) for name in right)


def grouped(records, threshold):
    groups = {}
    for name, stream in records:
        key = (tuple(stream['tokens']), tuple(stream['safe']))
        groups.setdefault(key, []).append(name)
    groups = list(groups.items())
    families = {}
    comparisons = 0
    for i, ((tokens, safe), names) in enumerate(groups):
        stream = {'tokens': tokens, 'safe': safe}
        # Identical streams share the diagonal-zero regions only across files.
        if len(names) > 1:
            first = 0
            for end in range(len(tokens) + 1):
                if end < len(tokens) and safe[end]:
                    continue
                if end - first >= threshold:
                    add(families, tokens, names, names, first, first, end)
                first = end + 1
        # Self-overlap remains eligible; never union distinct overlapping text.
        comparisons += 1
        for diagonal, first, end in verify.matches(stream, stream, threshold, True):
            add(families, tokens, names, names, first, first + diagonal, end)
        for (other_tokens, other_safe), other_names in groups[i + 1:]:
            comparisons += 1
            other = {'tokens': other_tokens, 'safe': other_safe}
            for diagonal, first, end in verify.matches(stream, other, threshold):
                add(families, tokens, names, other_names, first, first + diagonal, end)
    return families, comparisons, len(groups)


def exhaustive(records, threshold):
    families = {}
    for i, (name, stream) in enumerate(records):
        for j in range(i, len(records)):
            other_name, other = records[j]
            for diagonal, first, end in verify.matches(stream, other, threshold, i == j):
                add(families, stream['tokens'], [name], [other_name], first, first + diagonal, end)
    return families


def main():
    rows = []
    with tempfile.TemporaryDirectory(prefix='klin-family-') as directory:
        root = Path(directory)
        def stream(name, text, extension='ts'):
            path = root / (name + '.' + extension)
            path.write_text(text)
            return verify.normalized(path)
        source = 'function f() { ' + verify.body(30) + ' return x; }'
        normal = stream('normal', source)
        cases = {
            'identical': [('a', normal), ('b', normal), ('c', normal)],
            'renamed-interior': [('a', normal), ('b', stream('renamed', source.replace('f()', 'g()')))],
            'unsafe-boundary': [('a', stream('unsafe', source.replace('return x;', 'return /*\n*/ x;'))), ('b', normal)],
            'language-separated': [('a', normal), ('b', stream('rust', 'fn f() { ' + verify.body(30) + ' }', 'rs'))],
            'self-overlap': [('a', stream('repeated', 'function f() { ' + verify.body(20) * 3 + ' }'))],
        }
        # XY and YZ overlap in B; X and Z are different retained names.
        x, y, z = (' '.join(f'{v}{i}();' for i in range(16)) for v in 'xyz')
        cases['overlap-XY-YZ'] = [(n, stream(n, t)) for n, t in [('a', x + y), ('b', x + y + z), ('c', y + z)]]
        target = 'function f() { ' + verify.body(12) + ' x++; return x; }'
        target_stream = stream('target', target)
        selected = target_stream['positions'][0]
        gram = target_stream['tokens'][selected:selected + 41]
        decoys = []
        for attempt in range(5000):
            candidate = stream('decoy', target.replace('function f()', f'function decoy{attempt}()').replace('return x;', f'return value{attempt};'))
            if candidate['positions'] == [selected] and candidate['tokens'][selected:selected + 41] == gram:
                decoys.append((f'b{len(decoys):04}', candidate))
                if len(decoys) == 64:
                    break
        assert len(decoys) == 64
        cases['64-decoys-before-true-partner'] = [('a', target_stream), *decoys, ('z', target_stream)]
        for label, records in cases.items():
            actual, comparisons, groups = grouped(records, 60)
            expected = exhaustive(records, 60)
            assert actual == expected, (label, actual.keys(), expected.keys())
            rows.append({'case': label, 'records': len(records), 'classes': groups, 'representative_comparisons': comparisons, 'families': len(actual), 'occurrences': sum(map(len, actual.values())), 'oracle_equal': True})
        distinct = stream('distinct', 'function f() { ' + ' '.join(f'v{i} += {i};' for i in range(30)) + ' return x; }')
        records = [(f'copy{i:04}', distinct) for i in range(1000)]
        times = []
        timing = os.environ.get('FAMILY_TIMING', '1') != '0'
        for _ in range(5 if timing else 1):
            started = time.perf_counter() if timing else None
            families, comparisons, groups = grouped(records, 60)
            if timing:
                times.append((time.perf_counter() - started) * 1000)
        full = tuple(distinct['tokens'])
        assert len(families[full]) == 1000 and comparisons == groups == 1
        rows.append({'case': '1000-identical-normalized-records', 'records': 1000, 'classes': groups, 'representative_comparisons': comparisons, 'avoided_cross_file_pairs': 499500, 'whole_stream_occurrences': len(families[full]), 'grouping_ms': times, 'normalization_timed': False})
    output = {'scope': 'exact current-content families; ancestry unresolved; exhaustive fallback between distinct stream classes', 'cases': rows}
    text = json.dumps(output, indent=2) + '\n'
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(text)
    print(text)


if __name__ == '__main__':
    main()
