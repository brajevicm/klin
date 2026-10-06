"""Rust rule screen on the #480 design sets, before the fresh #494 scan.

    python3 design.py facts     # writes design-facts.json from the local corpora
    python3 design.py screen    # prints precision and recall per candidate
    python3 design.py rules DIR # writes DIR/rules.json with the frozen v5 decision

The four fully labeled sets are scored directly. The v3 blind sample is scored
with its sampling weights, once per reviewer.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
OLD = HERE.parent / 'duplication-speed-2026-10-05'
sys.path.insert(0, str(OLD))
import scope_exclusions  # noqa: E402
from holdout import overlapping  # noqa: E402

V3 = OLD / 'g4-fragments/blind-v3-2026-10-06'
SETS = {
    'calibration': Path('/tmp/klin490-corpus'),
    'calibration-holdout': Path('/tmp/klin480-holdout'),
    'calibration-holdout2': Path('/tmp/klin480-holdout2'),
    'calibration-introduced': Path('/tmp/klin480-intro/corpus'),
}
V3_CORPUS = Path('/tmp/klin480-g4-blind/out-combined/corpus')
FACTS = HERE / 'design-facts.json'


def source(corpus, span):
    if span['repo'] == 'klin':
        return corpus / 'klin/src' / span['path']
    return corpus / span['repo'] / span['path']


def unitclass(queries):
    target = os.environ.get('CARGO_TARGET_DIR', '/tmp/klin480-unitclass-target')
    subprocess.run(['cargo', 'build', '--release', '--offline', '--quiet', '--manifest-path', str(OLD / 'unitclass/Cargo.toml')],
                   check=True, env={**os.environ, 'CARGO_TARGET_DIR': target})
    out = subprocess.run([f'{target}/release/unitclass'], input=''.join(json.dumps(q) + '\n' for q in queries),
                         capture_output=True, text=True, check=True)
    return [json.loads(line) for line in out.stdout.splitlines()]


def facts_for(corpus, matches):
    scope_exclusions.CORPUS = corpus
    scope_exclusions.lines_of.__defaults__[0].clear()
    queries = [{'file': str(source(corpus, s)), 'start': s['start_line'], 'end': s['end_line']} for m in matches for s in m['spans']]
    rows = iter(unitclass(queries))
    result = {}
    for m in matches:
        spans = [next(rows) for _ in m['spans']]
        f = {**scope_exclusions.rules(m), 'tokens': m['tokens'], 'overlapping': overlapping(m),
             'unit_share': min(r['in_units'] / r['total'] if r['total'] else 0 for r in spans),
             'calls': min(r['calls'] for r in spans), 'control': min(r['control_or_await'] for r in spans),
             'statements': min(r['statements'] for r in spans)}
        prior = result.get(m['id'])
        if prior is None or f['tokens'] > prior['tokens']:
            result[m['id']] = f
    return result


def facts():
    out = {}
    for name, corpus in SETS.items():
        matches = [m for m in json.loads((OLD / name / 'mapping.json').read_text())['matches'] if m['language'] == 'Rust']
        out[name] = facts_for(corpus, matches)
    matches = [m for m in json.loads((V3 / 'population-mapping.json').read_text())['matches'] if m['language'] == 'Rust']
    sampled = {i for s in json.loads((V3 / 'sampling.json').read_text())['strata'] if s['language'] == 'Rust' for i in s['sample_ids']}
    out['v3-sample'] = facts_for(V3_CORPUS, [m for m in matches if m['id'] in sampled])
    FACTS.write_text(json.dumps(out, indent=1, sort_keys=True) + '\n')


def v2(f):
    return not f['overlapping'] and f['import_trim'] >= 100 and not f['trait_impl']


def behavior(f, floor):
    return (not f['overlapping'] and not f['trait_impl'] and not f['generated'] and f['import_trim'] >= floor
            and f['calls'] > 0 and f['control'] > 0 and f['statements'] >= 2)


def v5(f):
    if f['overlapping'] or f['trait_impl'] or f['import_trim'] < 60:
        return False
    return f['import_trim'] >= 100 or f['statements'] >= 2 and f['control'] >= 2


CANDIDATES = {
    'v2': v2,
    'v5 (frozen)': v5,
    **{f'v2 + behavior {t}': (lambda t: lambda f: v2(f) or behavior(f, t))(t) for t in (60, 70, 80, 90)},
    **{f'trim {t}': (lambda t: lambda f: not f['overlapping'] and f['import_trim'] >= t and not f['trait_impl'])(t) for t in (60, 80)},
}


def score(rows):
    kept = [w for keep, copy, w in rows if keep]
    tp = sum(w for keep, copy, w in rows if keep and copy)
    copies = sum(w for keep, copy, w in rows if copy)
    return f'{tp / sum(kept):.1%} / {tp / copies:.1%}' if kept and copies else 'n/a'


def screen():
    data = json.loads(FACTS.read_text())
    labels = {name: json.loads((OLD / name / 'labels.json').read_text()) for name in SETS}
    weights = {i: s['weight'] for s in json.loads((V3 / 'sampling.json').read_text())['strata'] if s['language'] == 'Rust' for i in s['sample_ids']}
    reviewers = {r: json.loads((V3 / f'review/results/reviewer-{r}.json').read_text()) for r in 'ab'}
    columns = [*SETS, 'pooled', 'v3 A', 'v3 B']
    print('| Candidate | ' + ' | '.join(columns) + ' |')
    print('|---|' + '---:|' * len(columns))
    for title, keep in CANDIDATES.items():
        cells, pooled = [], []
        for name in SETS:
            rows = [(keep(f), labels[name][i]['label'] == 'copy', 1) for i, f in data[name].items()]
            pooled += rows
            cells.append(score(rows))
        cells.append(score(pooled))
        for r in 'ab':
            cells.append(score([(keep(f), reviewers[r][i]['label'] == 'copy', weights[i]) for i, f in data['v3-sample'].items()]))
        print(f'| {title} | ' + ' | '.join(cells) + ' |')


def rules(directory):
    directory = Path(directory)
    matches = json.loads((directory / 'mapping.json').read_text())['matches']
    result = facts_for(directory / 'corpus', matches)
    for f in result.values():
        f['kept_v5'] = v5(f)
    (directory / 'rules.json').write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')


if __name__ == '__main__':
    {'facts': facts, 'screen': screen, 'rules': lambda: rules(sys.argv[2])}[sys.argv[1]]()
