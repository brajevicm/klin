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
