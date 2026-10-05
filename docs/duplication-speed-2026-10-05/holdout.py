"""Frozen per-language rules, measured on held-out labels.

Rust: imports and attributes trimmed (E2), no pair whose spans both sit in trait
impls (E3), T = 100. TypeScript: at least half of each span inside complete
functions, T = 60. Both were chosen on calibration/ before these labels existed.

    python3 holdout.py rules CORPUS DIR    # writes DIR/rules.json, before labels
    python3 holdout.py measure DIR         # joins DIR/labels.json afterwards
"""
import collections
import json
import os
import subprocess
import sys
from pathlib import Path

import scope_exclusions

HERE = Path(__file__).parent
UNIT_SHARE = 0.5
# A pair id names its two line spans. Matches that share them are one pair, kept if any is.
RULES = {'Rust': 100, 'TypeScript': 60}
LABELS = ('copy', 'boilerplate', 'required-shape', 'generated', 'distinct', 'mixed')


def unit_shares(corpus, matches):
    target = os.environ.get('CARGO_TARGET_DIR', '/tmp/klin480-unitclass-target')
    subprocess.run(['cargo', 'build', '--release', '--offline', '--quiet', '--manifest-path', str(HERE / 'unitclass/Cargo.toml')],
                   check=True, env={**os.environ, 'CARGO_TARGET_DIR': target})
    queries = [json.dumps({'file': str(corpus / s['repo'] / s['path']), 'start': s['start_line'], 'end': s['end_line']})
               for m in matches for s in m['spans']]
    out = subprocess.run([f'{target}/release/unitclass'], input='\n'.join(queries) + '\n', capture_output=True, text=True, check=True)
    rows = iter(json.loads(line) for line in out.stdout.splitlines())
    shares = {}
    for m in matches:
        share = min(r['in_units'] / r['total'] if r['total'] else 0 for r in [next(rows) for _ in m['spans']])
        shares[m['id']] = max(share, shares.get(m['id'], 0))
    return shares


def kept(match, facts):
    if match['language'] == 'Rust':
        return facts['import_trim'] >= RULES['Rust'] and not facts['trait_impl']
    return match['tokens'] >= RULES['TypeScript'] and facts['unit_share'] >= UNIT_SHARE


def rules(corpus, directory):
    scope_exclusions.CORPUS = corpus
    matches = json.loads((directory / 'mapping.json').read_text())['matches']
    shares = unit_shares(corpus, matches)
    result = {}
    for m in matches:
        facts = {**scope_exclusions.rules(m), 'unit_share': shares[m['id']], 'tokens': m['tokens']}
        facts['kept'] = kept(m, facts)
        if m['id'] not in result or facts['kept'] and not result[m['id']]['kept']:
            result[m['id']] = facts
    (directory / 'rules.json').write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
    print(collections.Counter((m['language'], result[m['id']]['kept']) for m in matches))


def measure(directory):
    matches = list({m['id']: m for m in json.loads((directory / 'mapping.json').read_text())['matches']}.values())
    facts = json.loads((directory / 'rules.json').read_text())
    labels = json.loads((directory / 'labels.json').read_text())
    assert set(labels) == {m['id'] for m in matches}, 'every pair must be labeled'
    assert all(v['label'] in LABELS for v in labels.values())
    is_copy = lambda m: labels[m['id']]['label'] == 'copy'
    result = {}
    for language, threshold in RULES.items():
        rows = [m for m in matches if m['language'] == language]
        at_threshold = [m for m in rows if facts[m['id']]['tokens'] >= threshold]
        repos = sorted({m['spans'][0]['repo'] for m in rows})
        def score(selected):
            copies = sum(map(is_copy, selected))
            return {'copy': copies, 'kept': len(selected), 'precision': round(copies / len(selected), 3) if selected else None}
        keep = [m for m in rows if facts[m['id']]['kept']]
        result[language] = {
            'rule': score(keep),
            'passes_80': bool(keep) and sum(map(is_copy, keep)) / len(keep) >= 0.8,
            'recall': {'copies_kept': sum(map(is_copy, keep)), 'copies_all': sum(map(is_copy, rows))},
            'baseline_at_T': score(at_threshold),
            'by_repo': {r: score([m for m in keep if m['spans'][0]['repo'] == r]) for r in repos},
            'non_copy_kept': collections.Counter(labels[m['id']]['label'] for m in keep if not is_copy(m)),
            'labels_all': collections.Counter(labels[m['id']]['label'] for m in rows),
        }
    (directory / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    if sys.argv[1] == 'rules':
        rules(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        measure(Path(sys.argv[2]))
