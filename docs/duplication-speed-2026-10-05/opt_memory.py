"""Serial OS peak-RSS comparison; one isolated child per sample (macOS bytes)."""
import json
from pathlib import Path
import statistics
import subprocess
import sys

HERE = Path(__file__).resolve().parent
WRAPPER = '''import json,resource,subprocess,sys
row=json.loads(subprocess.check_output(sys.argv[1:]))
print(json.dumps({'query':row,'peak_rss_bytes':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss}))'''


def main():
    assert sys.platform == 'darwin', 'ru_maxrss units here are macOS bytes'
    binaries = {'o1': '/tmp/klin490-opt-o1/dup-speed',
                'o1_o2': str(HERE / 'proto/target/release/dup-speed')}
    rows = []
    for case, root, count, selection in [('klin-largest-64', 'klin/src', 20, 'largest'),
                                         ('klin-largest-64', 'klin/src', 100, 'largest'),
                                         ('changed-100k', 'changed-100k', 20, 'spread')]:
        for iteration in range(5):
            order = list(binaries.items())
            if iteration % 2:
                order.reverse()
            for name, binary in order:
                row = json.loads(subprocess.check_output([sys.executable, '-c', WRAPPER,
                    binary, 'query', '/tmp/klin490-corpus/' + root,
                    '/tmp/klin490-measured/' + case, str(count), '60', selection, 'stop']))
                row.update(candidate=name, case=case + '-' + str(count), iteration=iteration)
                rows.append(row)
    summary = {}
    for case in sorted({r['case'] for r in rows}):
        summary[case] = {name: {'median_peak_rss_bytes': statistics.median(
            r['peak_rss_bytes'] for r in rows if r['case'] == case and r['candidate'] == name)}
            for name in binaries}
    (HERE / 'results-optimizations/memory.json').write_text(
        json.dumps({'rows': rows, 'summary': summary}, indent=2) + '\n')


if __name__ == '__main__':
    main()
