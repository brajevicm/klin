"""Check frozen headline evidence and optionally reproduce deterministic outcomes."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from corpus import cases

here = Path(__file__).parent.resolve()
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--reproduce', action='store_true')
args = parser.parse_args()
rows = json.loads((here / 'raw/results.json').read_text())
env = json.loads((here / 'raw/environment.json').read_text())
histories = json.loads((here / 'raw/trajectory.json').read_text())
assert len(rows) == 288
assert env['corpus_id'] == hashlib.sha256(json.dumps(cases(), sort_keys=True, separators=(',', ':')).encode()).hexdigest()
for arm, calls, search, unique, harm in [('A', 376, 216, 132, 56),
                                       ('B', 420, 276, 120, 56), ('C', 120, 8, 8, 8)]:
    selected = [r for r in rows if r['arm'] == arm]
    assert sum(r['oracle_executions'] for r in selected) == calls
    assert sum(r['groups_considered'] for r in selected) == search
    assert sum(r['unique_search_candidates'] for r in selected) == unique
    assert sum(r['harmful'] for r in selected) == harm
for row in rows:
    search = [a for a in row['attempts'] if a['evidence'] == 'oracle'
              and (isinstance(a['candidate'], list) and a['candidate'][0] != 'audit'
                   or a['candidate'] == 'collapse-identity-boundary')]
    assert row['unique_search_candidates'] == len({a['tree'] for a in search})
    assert row['duplicate_search_candidates'] == len(search) - row['unique_search_candidates']
    assert row['trajectory_bytes'] == len(json.dumps(histories[row['task']], separators=(',', ':')).encode())
    assert all(a['scratch_cleaned'] for a in row['attempts'])
    assert row['removed_cost'] == row['before_cost'] - row['after_cost']
    case = next(c for c in cases() if c['name'] == row['task'])
    basis = dict(text=case['oracle'], runtime=env['runtimes'][case['repo']],
                 repeats=3 if case['name'] == 'flaky' else 1,
                 timeout=env['timeout'], output=env['output_bound'])
    assert row['oracle_id'] == hashlib.sha256(json.dumps(basis, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
with tempfile.TemporaryDirectory() as out:
    root = Path(out)
    (root / 'results.json').write_bytes((here / 'raw/results.json').read_bytes())
    subprocess.run([sys.executable, str(here / 'summarize.py'), str(root)],
                   check=True, stdout=subprocess.DEVNULL)
    assert (root / 'summary.tsv').read_bytes() == (here / 'raw/summary.tsv').read_bytes(), 'summary.tsv differs from summarize.py output'
print('frozen headline counts, evidence basis, corpus, storage and summary.tsv assertions pass')
if args.reproduce:
    with tempfile.TemporaryDirectory() as out:
        subprocess.run([sys.executable, str(here / 'run.py'), '--trajectory',
                        str(here / 'raw/trajectory.json'), '--output', out], check=True)
        fresh = json.loads((Path(out) / 'results.json').read_text())
        # Runtime versions/timing are a new basis on CI; compare only deterministic
        # fixture outcomes, never pool durations or claim identical tool versions.
        keys = ('task', 'arm', 'order', 'atomization', 'repetition', 'initial',
                'groups_considered', 'audit_groups', 'unique_search_candidates',
                'duplicate_search_candidates', 'unique_oracle_candidates',
                'oracle_executions', 'independent_executions', 'before_cost',
                'after_cost', 'removed_cost', 'one_minimal', 'independent',
                'independent_before', 'harmful', 'inconclusive', 'final_tree',
                'retained', 'trajectory_bytes', 'surfaced_candidates', 'transformed')
        assert len(fresh) == len(rows)
        for old, new in zip(rows, fresh):
            assert {k: old[k] for k in keys} == {k: new[k] for k in keys}, (old['task'], old['arm'])
        assert json.loads((Path(out) / 'environment.json').read_text())['corpus_id'] == env['corpus_id']
    print('full retained-history reproduction matches frozen deterministic outcomes')
