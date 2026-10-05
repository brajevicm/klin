"""Serial, interleaved attribution against frozen REG2 indexes; no full-hook claim."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
BINS = {
    'reference': Path('/tmp/klin490-opt-reference/dup-speed'),
    'o1': Path('/tmp/klin490-opt-o1/dup-speed'),
    'o1_o2': HERE / 'proto/target/release/dup-speed',
}
CASES = [
    ('1m-64', '1m', 20, 'spread'),
    ('1m-64', '1m', 100, 'spread'),
    ('klin-largest-64', 'klin/src', 20, 'largest'),
    ('klin-largest-64', 'klin/src', 100, 'largest'),
    ('changed-100k', 'changed-100k', 20, 'spread'),
    ('glaredb-64', 'glaredb', 100, 'spread'),
]


def main():
    output = HERE / 'results-optimizations'
    rows = []
    with (output / 'warm.jsonl').open('w') as file:
        for index, root, n, selection in CASES:
            for iteration in range(5):
                # Reverse alternate iterations to avoid always favoring a warm successor.
                order = list(BINS.items())
                if iteration % 2:
                    order.reverse()
                pair = []
                for name, binary in order:
                    row = json.loads(subprocess.check_output([
                        str(binary), 'query', '/tmp/klin490-corpus/' + root,
                        '/tmp/klin490-measured/' + index, str(n), '60', selection, 'stop']))
                    row.update(candidate=name, case=index + '-' + str(n), iteration=iteration)
                    pair.append(row)
                    rows.append(row)
                    file.write(json.dumps(row) + '\n')
                    file.flush()
                semantic = lambda r: {k: v for k, v in r.items()
                                      if not k.endswith('_ms') and k != 'candidate'}
                assert all(semantic(r) == semantic(pair[0]) for r in pair), pair
    summary = {'binary_sha256': {k: hashlib.sha256(v.read_bytes()).hexdigest()
                                for k, v in BINS.items()}, 'cases': {}}
    for case in sorted({r['case'] for r in rows}):
        summary['cases'][case] = {}
        for name in BINS:
            subset = [r for r in rows if r['case'] == case and r['candidate'] == name]
            summary['cases'][case][name] = {
                metric: {'median': statistics.median(r[metric] for r in subset),
                         'max': max(r[metric] for r in subset)}
                for metric in ('design_c_ms', 'stream_ms', 'read_parse_ms')}
    (output / 'warm-summary.json').write_text(json.dumps(summary, indent=2) + '\n')


if __name__ == '__main__':
    main()
