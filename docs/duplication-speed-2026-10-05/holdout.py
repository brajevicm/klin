"""Frozen per-language rules, measured on held-out labels.

Rust: imports and attributes trimmed (E2), no pair whose spans both sit in trait
impls (E3), T = 100. TypeScript: at least half of each span inside complete
functions, T = 60. Both were chosen on calibration/ (version 1).

Version 2, chosen after calibration-holdout/ was read: no pair whose two spans
overlap in one file, and TypeScript also needs NON_JSX tokens outside JSX.

Version 3 is the G4 candidate: keep version 2 whole-unit TypeScript pairs, or
allow a fragment with the same NON_JSX floor when both spans have a call and
control flow or await and at least two complete statements.

    python3 holdout.py rules CORPUS DIR    # writes DIR/rules.json, before labels
    python3 holdout.py measure DIR [v2|v3]
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
NON_JSX = 60
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
    shares, non_jsx, calls, control_or_await, statements = {}, {}, {}, {}, {}
    for m in matches:
        spans = [next(rows) for _ in m['spans']]
        shares[m['id'], m['family']] = min(r['in_units'] / r['total'] if r['total'] else 0 for r in spans)
        non_jsx[m['id'], m['family']] = m['tokens'] * min((r['total'] - r['jsx']) / r['total'] if r['total'] else 0 for r in spans)
        calls[m['id'], m['family']] = min(r['calls'] for r in spans)
        control_or_await[m['id'], m['family']] = min(r['control_or_await'] for r in spans)
        statements[m['id'], m['family']] = min(r['statements'] for r in spans)
    return shares, non_jsx, calls, control_or_await, statements


def overlapping(match):
    a, b = match['spans']
    return (a['repo'], a['path']) == (b['repo'], b['path']) and a['start_line'] <= b['end_line'] and b['start_line'] <= a['end_line']


def kept(match, facts):
    if overlapping(match):
        return False
    if match['language'] == 'Rust':
        return facts['import_trim'] >= RULES['Rust'] and not facts['trait_impl']
    return match['tokens'] >= RULES['TypeScript'] and facts['unit_share'] >= UNIT_SHARE and facts['non_jsx'] >= NON_JSX


def kept_v3(match, facts):
    if overlapping(match):
        return False
    if match['language'] == 'Rust':
        return kept(match, facts)
    return (match['tokens'] >= RULES['TypeScript'] and facts['non_jsx'] >= NON_JSX
            and (facts['unit_share'] >= UNIT_SHARE
                 or facts['calls'] > 0 and facts['control_or_await'] > 0
                 and facts['statements'] >= 2))


def rules(corpus, directory):
    scope_exclusions.CORPUS = corpus
    matches = json.loads((directory / 'mapping.json').read_text())['matches']
    shares, non_jsx, calls, control_or_await, statements = unit_shares(corpus, matches)
    result = {}
    for m in matches:
        facts = {**scope_exclusions.rules(m), 'unit_share': shares[m['id'], m['family']], 'non_jsx': non_jsx[m['id'], m['family']], 'calls': calls[m['id'], m['family']], 'control_or_await': control_or_await[m['id'], m['family']], 'statements': statements[m['id'], m['family']], 'tokens': m['tokens']}
        facts['kept'] = kept(m, facts)
        facts['kept_v3'] = kept_v3(m, facts)
        if m['id'] not in result:
            result[m['id']] = facts
        else:
            result[m['id']]['kept'] |= facts['kept']
            result[m['id']]['kept_v3'] |= facts['kept_v3']
    (directory / 'rules.json').write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
    print('v2', collections.Counter((m['language'], result[m['id']]['kept']) for m in matches))
    print('v3', collections.Counter((m['language'], result[m['id']]['kept_v3']) for m in matches))


def measure(directory, version='v2'):
    keep_key = {'v2': 'kept', 'v3': 'kept_v3'}.get(version)
    if keep_key is None:
        raise ValueError('version must be v2 or v3')
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
        keep = [m for m in rows if facts[m['id']][keep_key]]
        result[language] = {
            'version': version,
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
        measure(Path(sys.argv[2]), sys.argv[3] if len(sys.argv) > 3 else 'v2')
