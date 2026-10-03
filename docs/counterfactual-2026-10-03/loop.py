"""Re-run reduction on actual intent-restored AX source, including its whitespace."""
import copy
import json
from pathlib import Path
import tempfile

from corpus import cases
import run

here = Path(__file__).parent
run.RUNTIMES = json.loads((here / 'raw/environment.json').read_text())['runtimes']
rows = []
for arm in 'AC':
    case = copy.deepcopy(next(c for c in cases() if c['name'] == 'boundary'))
    repair = json.loads((here / 'ax' / f'klin356-ax-{arm}.json').read_text())
    restored = repair['repaired_files']
    source = restored['router.mjs']
    prefix = case['segments'][0]['after']
    suffix = case['segments'][2]['after']
    assert source.startswith(prefix) and source.endswith(suffix)
    case['segments'][1]['after'] = source[len(prefix):-len(suffix)]
    case['segments'][1]['turn'] = 2
    retained = {i for i, a in enumerate(case['segments']) if a['before'] != a['after']}
    assert run.tree(case, retained) == restored
    with tempfile.TemporaryDirectory() as out:
        row = run.experiment(case, arm, 'forward', 'semantic', 0, Path(out), 1, json.loads(json.dumps(run.trajectory(case))))
    packet = json.loads((here / 'ax' / f'klin356-repair-{arm}.json').read_text())
    assert row['final_tree'] == run.digest(packet['final_files'])
    assert row['removed_cost'] > 0
    packet.update(current_files=restored, repeat_candidate_tree=row['final_tree'],
                  restored_source_equivalence_checked=True,
                  feedback='REVIEW repeated experiment: the same identity-collapse candidate remains removable under the frozen behavioral oracle. The current source retains the requested adaptation boundary. Preserve intent; do not edit merely to clear this observation.')
    (here / 'ax' / f'klin356-repeat-{arm}.json').write_text(json.dumps(packet, indent=2) + '\n')
    rows.append(row)
(here / 'raw/loop-results.json').write_text('[\n' + ',\n'.join(json.dumps(r, separators=(',', ':')) for r in rows) + '\n]\n')
print('exact restored-source experiment repeats the same boundary removal proposal twice')
