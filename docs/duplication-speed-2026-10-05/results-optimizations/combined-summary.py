"""Summarize five same-binary off/on pairs and enforce stable enabled counters."""
import json
from pathlib import Path
import statistics

HERE = Path(__file__).resolve().parent
rows = [json.loads(line.removeprefix('DUP_COMBINED_AB '))
        for line in (HERE / 'combined.log').read_text().splitlines()
        if line.startswith('DUP_COMBINED_AB ')]
assert len(rows) == 30, len(rows)
summary = {}
for phase in ('warm20', 'warm100', 'cold'):
    pairs = []
    for iteration in range(5):
        off = next(r for r in rows if r['phase'] == phase and r['iteration'] == iteration and not r['enabled'])
        on = next(r for r in rows if r['phase'] == phase and r['iteration'] == iteration and r['enabled'])
        assert off['counters']['shared_files'] == off['counters']['normalize_ms'] == 0
        assert on['counters']['extra_parses'] == on['counters']['unchanged_source_reads'] == 0
        pairs.append((off, on))
    for key in ('shared_files', 'tokens', 'fingerprints', 'checksum', 'reuse_hits', 'derived_inputs', 'retained_payload_bytes'):
        assert len({on['counters'][key] for off, on in pairs}) == 1, (phase, key)
    summary[phase] = {
        'off_median_ms': statistics.median(off['end_to_end_ms'] for off, on in pairs),
        'on_median_ms': statistics.median(on['end_to_end_ms'] for off, on in pairs),
        'paired_median_delta_ms': statistics.median(on['end_to_end_ms'] - off['end_to_end_ms'] for off, on in pairs),
        'paired_median_delta_percent': statistics.median(100 * (on['end_to_end_ms'] / off['end_to_end_ms'] - 1) for off, on in pairs),
        'paired_max_delta_percent': max(100 * (on['end_to_end_ms'] / off['end_to_end_ms'] - 1) for off, on in pairs),
        'pairs_over_5_percent': sum(on['end_to_end_ms'] > off['end_to_end_ms'] * 1.05 for off, on in pairs),
        'normalize_median_ms': statistics.median(on['counters']['normalize_ms'] for off, on in pairs),
        'retained_payload_bytes': pairs[0][1]['counters']['retained_payload_bytes'],
        'reuse_hits': pairs[0][1]['counters']['reuse_hits'],
    }
(HERE / 'combined.json').write_text(json.dumps({'rows': rows, 'summary': summary}, indent=2) + '\n')
print(json.dumps(summary, indent=2))
