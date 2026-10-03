"""Recheck the recorded AX candidates under the frozen corpus evidence."""
import json
from pathlib import Path
import tempfile

from corpus import cases
from run import run_evidence

here = Path(__file__).parent
results = []
for arm in 'ABC':
    packet = json.loads((here / 'ax' / f'klin356-repair-{arm}.json').read_text())
    repair = json.loads((here / 'ax' / f'klin356-ax-{arm}.json').read_text())
    case = next(c for c in cases() if c['name'] == packet['task'])
    files = repair.get('repaired_files', packet['final_files'])
    with tempfile.TemporaryDirectory() as work:
        oracle = run_evidence(case, files, case['oracle'], Path(work) / 'state', 1)
        holdout = run_evidence(case, files, case['independent'], Path(work) / 'state', 1)
    assert oracle['state'] == holdout['state'] == 'pass'
    results.append(dict(arm=arm, task=case['name'], action=repair['action'],
                        packet_bytes=len((here / 'ax' / f'klin356-repair-{arm}.json').read_bytes()),
                        research_agent_response_turns=1, host_turns=None,
                        requested_intent_preserved=(arm == 'B' or 'adapt' in files['router.mjs']),
                        oracle=oracle, independent=holdout))
(here / 'raw' / 'ax-verification.json').write_text(json.dumps(results, indent=2) + '\n')
print('three recorded AX candidates pass the frozen oracle and independent holdout')
