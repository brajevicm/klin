from pathlib import Path
import json,statistics
p=Path(__file__).resolve().parent
rows=[json.loads(line.removeprefix('DUP_REUSE_AB ')) for line in (p/'reuse.log').read_text().splitlines() if line.startswith('DUP_REUSE_AB ')]
assert len(rows)==30,len(rows)
summary={}
for phase in ('warm20','warm100','cold'):
    pairs=[]
    for i in range(5):
        a=next(r for r in rows if r['phase']==phase and r['iteration']==i and not r['enabled'])
        b=next(r for r in rows if r['phase']==phase and r['iteration']==i and r['enabled'])
        for key in ('shared_files','tokens','fingerprints','checksum','extra_parses','unchanged_source_reads'):
            assert a['counters'][key]==b['counters'][key],(phase,i,key)
        pairs.append((a,b))
    summary[phase]={
        'no_reuse_median_ms':statistics.median(a['end_to_end_ms'] for a,b in pairs),
        'reuse_median_ms':statistics.median(b['end_to_end_ms'] for a,b in pairs),
        'paired_median_delta_ms':statistics.median(b['end_to_end_ms']-a['end_to_end_ms'] for a,b in pairs),
        'no_reuse_normalize_median_ms':statistics.median(a['counters']['normalize_ms'] for a,b in pairs),
        'reuse_normalize_median_ms':statistics.median(b['counters']['normalize_ms'] for a,b in pairs),
        'reuse_hits':sorted({b['counters']['reuse_hits'] for a,b in pairs}),
        'derived_inputs':sorted({b['counters']['derived_inputs'] for a,b in pairs}),
        'retained_payload_bytes':sorted({b['counters']['retained_payload_bytes'] for a,b in pairs}),
        'pair_semantics_equal':True,
    }
(p/'reuse.json').write_text(json.dumps({'rows':rows,'summary':summary},indent=2)+'\n')
print(json.dumps(summary,indent=2))
