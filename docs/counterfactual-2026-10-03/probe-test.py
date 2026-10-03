"""Black-box checks for the disposable experiment's command-line seam."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
with tempfile.TemporaryDirectory() as out:
    subprocess.run([sys.executable, str(HERE / 'run.py'), '--task', 'bug',
                    '--repeat', '1', '--output', out], check=True)
    rows = json.loads((Path(out) / 'results.json').read_text())
    assert len(rows) == 12  # three arms, two orders, two atomizations
    assert all(row['oracle_id'] == rows[0]['oracle_id'] for row in rows)
    assert all(row['removed_cost'] > 0 and not row['harmful'] for row in rows if row['arm'] in ('A', 'B'))
    assert all(row['groups_considered'] == 0 for row in rows if row['arm'] == 'C')
    assert all(row['external_seconds'] > 0 and row['analysis_seconds'] > 0 for row in rows)
print('bug reduction preserves the independent negative-input contract')
with tempfile.TemporaryDirectory() as out:
    subprocess.run([sys.executable, str(HERE / 'run.py'), '--security', '--output', out], check=True)
    rows = json.loads((Path(out) / 'security.json').read_text())
    assert rows['timeout']['state'] == 'inconclusive'
    assert rows['output']['state'] == 'inconclusive'
    assert rows['drift']['state'] == 'inconclusive'
    assert rows['descendant_inherited_pipe']['state'] == 'inconclusive'
    assert rows['descendant_closed_pipe']['state'] == 'pass'
    assert not rows['descendant_survived']
    assert rows['cleanup_complete'] and rows['dirty_caller_preserved']
print('bounded execution, descendant termination, drift, cleanup, and caller isolation pass')
with tempfile.TemporaryDirectory() as out:
    subprocess.run([sys.executable, str(HERE / 'run.py'), '--task', 'flaky',
                    '--repeat', '1', '--output', out], check=True)
    rows = json.loads((Path(out) / 'results.json').read_text())
    assert all(r['initial'] == 'inconclusive' and r['groups_considered'] == 0 for r in rows)
    assert all(not r['one_minimal'] and r['independent'] == 'pass' for r in rows)
print('unstable evidence never authorizes removal or one-minimality')
# Search work is reported both as attempts and as distinct candidate trees.
with tempfile.TemporaryDirectory() as out:
    subprocess.run([sys.executable, str(HERE / 'run.py'), '--task', 'paired-history',
                    '--repeat', '1', '--output', out], check=True)
    rows = json.loads((Path(out) / 'results.json').read_text())
    assert all(r['unique_search_candidates'] <= r['groups_considered'] for r in rows)
    assert all(r['removed_cost'] == (4 if r['arm'] == 'B' else 0) for r in rows)
    history = json.loads((Path(out) / 'trajectory.json').read_text())
    history['paired-history'][0]['groups'] = []
    bad = Path(out) / 'bad-history.json'
    bad.write_text(json.dumps(history))
    rejected = subprocess.run([sys.executable, str(HERE / 'run.py'), '--task', 'paired-history',
                               '--repeat', '1', '--trajectory', str(bad), '--output', out],
                              capture_output=True, text=True)
    assert rejected.returncode != 0 and 'trajectory' in rejected.stderr
print('paired removal uses retained history and rejects corrupted replay data')
with tempfile.TemporaryDirectory() as out:
    # Valid history that separates the coordinated rename into two turns.
    history = json.loads((HERE / 'raw/trajectory.json').read_text())
    first, second = history['paired-history']
    gap = '\n' + '\n'.join('// separator ' + str(i) for i in range(8)) + '\n'
    middle = {'router.mjs': 'const deliver = s => s;\n' + gap +
              'export const route = s => send(s);\n' + gap +
              'export const normalize = s => s;\n'}
    import hashlib
    fingerprint = hashlib.sha256(json.dumps(middle, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    history['paired-history'] = [
        dict(turn=1, before=first['before'], after=fingerprint, groups=[0]),
        dict(turn=2, before=fingerprint, after=first['after'], groups=[2]),
        dict(turn=3, before=second['before'], after=second['after'], groups=[4])]
    path = Path(out) / 'split.json'
    path.write_text(json.dumps(history))
    subprocess.run([sys.executable, str(HERE / 'run.py'), '--task', 'paired-history',
                    '--repeat', '1', '--trajectory', str(path), '--output', out], check=True)
    rows = json.loads((Path(out) / 'results.json').read_text())
    assert all(r['removed_cost'] == 0 for r in rows)
print('valid retained grouping changes B search outcome without changing corpus turns')
# The rendered summary is part of the verified archive, not just raw results.
with tempfile.TemporaryDirectory() as out:
    import shutil
    copied = Path(out) / 'research'
    shutil.copytree(HERE, copied)
    (copied / 'raw/summary.tsv').write_text('stale summary\n')
    rejected = subprocess.run([sys.executable, str(copied / 'check-results.py')],
                              capture_output=True, text=True)
    assert rejected.returncode != 0 and 'summary.tsv' in rejected.stderr
print('archive verification rejects a stale rendered summary')
