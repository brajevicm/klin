"""Draw the blind #494 sample from a scan with frozen v5 decisions.

    python3 sample.py OUT    # OUT holds the scan's mapping.json, corpus/ and rules.json

Writes sampling.json, population-mapping.json and review/ next to this file.
"""
import json
import random
import sys
from pathlib import Path

HERE = Path(__file__).parent
SEED = 20261007
KEPT_CAP, TOTAL = 60, 150
URLS = {'starship': 'https://github.com/starship/starship', 'helix': 'https://github.com/helix-editor/helix',
        'nushell': 'https://github.com/nushell/nushell'}


def main(out):
    out = Path(out)
    matches = list({m['id']: m for m in json.loads((out / 'mapping.json').read_text())['matches']}.values())
    rules = json.loads((out / 'rules.json').read_text())
    rng = random.Random(SEED)
    strata = {}
    for m in sorted(matches, key=lambda m: m['id']):
        repo = m['spans'][0]['repo'].split('@')[0]
        strata.setdefault((repo, rules[m['id']]['kept_v5']), []).append(m['id'])
    sizes = {key: min(len(ids), KEPT_CAP) for key, ids in strata.items() if key[1]}
    dropped = {key: ids for key, ids in strata.items() if not key[1]}
    budget = TOTAL - sum(sizes.values())
    population = sum(map(len, dropped.values()))
    for key, ids in dropped.items():
        sizes[key] = min(len(ids), max(1, round(budget * len(ids) / population)))
    rows, sampled = [], []
    for (repo, kept), ids in sorted(strata.items()):
        chosen = sorted(rng.sample(ids, sizes[repo, kept]))
        sampled += chosen
        rows.append({'language': 'Rust', 'repo': repo, 'candidate_kept': kept, 'population_n': len(ids),
                     'sample_n': len(chosen), 'weight': len(ids) / len(chosen), 'sample_ids': chosen})
    (HERE / 'sampling.json').write_text(json.dumps({
        'seed': SEED, 'created': '2026-10-07', 'rule': 'v5 in design.py, frozen in 07516cf5',
        'method': f'Census of v5-kept pairs up to {KEPT_CAP} per repository; dropped pairs sampled in proportion to repository size, {TOTAL} pairs in all.',
        'estimator': {'stratum_weight': 'population_n / sample_n', 'positive_label': 'copy only'},
        'strata': rows}, indent=1) + '\n')
    (HERE / 'population-mapping.json').write_text(json.dumps({'matches': matches}, indent=1) + '\n')
    by_id = {m['id']: m for m in matches}
    packet = []
    for pair_id in sorted(sampled):
        m = by_id[pair_id]
        locations = []
        for span in m['spans']:
            lines = (out / 'corpus' / span['repo'] / span['path']).read_text().splitlines()
            repo = span['repo'].split('@')[0]
            locations.append({'repo': repo, 'path': span['path'], 'language': 'Rust', 'start_line': span['start_line'],
                              'end_line': span['end_line'],
                              'url': f"{URLS[repo]}/blob/{m['commit']}/{span['path']}#L{span['start_line']}-L{span['end_line']}",
                              'context': '\n'.join(lines[max(0, span['start_line'] - 4):span['end_line'] + 3])})
        packet.append({'id': pair_id, 'locations': locations})
    review = HERE / 'review'
    review.mkdir(exist_ok=True)
    (review / 'review-packet.json').write_text(json.dumps(packet, indent=1) + '\n')
    print({f'{r["repo"]}:{"kept" if r["candidate_kept"] else "dropped"}': (r['sample_n'], r['population_n']) for r in rows})


if __name__ == '__main__':
    main(sys.argv[1])
