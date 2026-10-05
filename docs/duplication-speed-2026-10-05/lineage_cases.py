"""Validate certificate accounting, not infer region ancestry from token offsets."""
import json


def verdict(sources, new=0, unknown=False):
    return {'regressions_floor': max(0, len(set(sources)) + new - 1), 'incomplete': unknown}


CASES = [
    ('synchronized-certified-body-edit', ['H1', 'H1'], 0, False, 0),
    ('independent-certified-convergence', ['H1', 'H2'], 0, False, 1),
    ('third-copy-after-synchronized-extension', ['H1', 'H1'], 1, False, 1),
    ('split-without-identity-certificate', [], 0, True, 0),
    ('eligibility-exit-consumes-slot', ['H1'], 1, False, 1),
    ('competing-removed-and-edited-ancestry', ['H1'], 0, True, 0),
    ('proven-origins-survive-ambiguity', ['H1', 'H2'], 0, True, 1),
    ('exact-replacement-cardinality', ['H1'], 1, False, 1),
    ('cross-file-edit-unsupported', [], 0, True, 0),
]

if __name__ == '__main__':
    rows = []
    for label, sources, new, unknown, floor in CASES:
        result = verdict(sources, new, unknown)
        assert result['regressions_floor'] == floor
        rows.append({'case': label, 'certified_sources': sources, 'new_count': new, **result})
    # A slot may be consumed once; only absent, competition-free slots are supply.
    slots = {'A': 'consumed', 'B': 'unresolved', 'C': 'free'}
    assert sum(state == 'free' for state in slots.values()) == 1
    assert 'B' not in [slot for slot, state in slots.items() if state == 'free']
    print(json.dumps({'scope': 'accounting with supplied certificates; not an identity implementation', 'cases': rows}, indent=2))
