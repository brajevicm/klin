"""Synthetic repair-pressure probes; no agent prevalence/repair-success claim."""
import json
import sys
import tempfile
from pathlib import Path
from verify import BIN, body, run


def main():
    source = 'function f(x: number) { ' + body(40) + ' return x; }'
    mutations = [
        ('local-parameter-rename', source.replace('x', 'result'), 'out of scope by design; routine cheap equality evasion'),
        ('literal-neutral-perturbation', source.replace('x += ', 'x += 0 + '), 'superficial appeasement'),
        ('comments-formatting', source.replace(';', '; /* note */\n'), 'still detected'),
        ('dead-insertion', source.replace(';', '; void 0;'), 'superficial appeasement'),
        ('logging-insertion', source.replace(';', '; console.log("trace");'), 'superficial appeasement; adds observable telemetry'),
        ('commutative-statement-reorder', 'function f(x: number) { ' + ' '.join(f'x += {i};' for i in reversed(range(40))) + ' return x; }', 'superficial appeasement for this commutative fixture'),
        ('wrapper', 'function wrapper(x: number) { return g(x); }\n' + source.replace('f(', 'g('), 'still detected'),
        ('reuse-helper', 'function g(x: number) { return f(x); }', 'legitimate repair if the existing f is accessible'),
        ('inline-helper', source.replace('f(', 'g('), 'still detected'),
        ('split-across-helpers', '\n'.join('function h%d(x: number) { %s return x; }' % (i, ' '.join(f'x += {j};' for j in range(i*8, (i+1)*8))) for i in range(5)), 'superficial appeasement; no reuse introduced'),
        ('split-below-threshold', '\n'.join('function h%d(x: number) { %s return x; }' % (i, ' '.join(f'x += {j};' for j in range(i*8, (i+1)*8))) for i in range(5)), 'superficial appeasement'),
        ('combine-small-helpers', source.replace('f(', 'combined('), 'still detected'),
        ('branch-reshape', source.replace('x += ', 'if (true) x += '), 'superficial appeasement'),
        ('equivalent-expression', source.replace('x += ', 'x = x + '), 'superficial appeasement'),
        ('move-rename-function', source.replace('f(', 'moved('), 'still detected'),
        ('tiny-neutral-difference', source.replace('return x;', 'void 0; return x;'), 'still detected; large copied prefix remains'),
        ('test-path', source.replace('f(', 'g('), 'out of scope by design; lineage must not release debt'),
        ('generated-looking-path', source.replace('f(', 'g('), 'still detected; no generated-code exclusion is specified'),
    ]
    rows, intentional = [], []
    with tempfile.TemporaryDirectory(prefix='klin-dup-appeasement-') as directory:
        root = Path(directory)
        for label, after, classification in mutations:
            case = root / label
            case.mkdir()
            (case / 'a.ts').write_text(source)
            path = 'copy.test.ts' if label == 'test-path' else 'generated/copy.ts' if label == 'generated-looking-path' else 'copy.ts'
            copy = case / path
            copy.parent.mkdir(parents=True, exist_ok=True)
            copy.write_text(source.replace('f(', 'g('))
            index = root / (label + '-index')
            run('build', case, index)
            copy.write_text(after)
            manifest = root / (label + '.txt')
            manifest.write_text(path + '\n')
            result = run('query', case, index, 1, 60, 'spread', 'full', manifest)
            detected = result['proven'] + result['check_blocked'] > 0
            expected = classification.startswith('still detected')
            assert detected == expected, (label, classification, result)
            rows.append({'attack': label, 'detected': detected, 'incomplete': result['incomplete'],
                         'classification': classification})
        for label, reason in (
            ('protocol-shape', 'Versioned wire encoders have separate compatibility obligations; extracting shared state requires protocol review.'),
            ('independent-adapters', 'Adapters belong to independently deployed integrations; shared code would add a deployment dependency.'),
            ('migration-coexistence', 'Old and new schema migrations coexist; historical migration behavior must remain fixed.'),
            ('independent-evolution', 'Initially identical tenant policy implementations intentionally evolve independently.'),
            ('generated-implementations', 'A generator owns both outputs; changes belong in its template rather than output extraction.'),
        ):
            case = root / label
            case.mkdir()
            (case / 'legacy.ts').write_text('// ' + reason + '\n' + source)
            (case / 'new.ts').write_text('// ' + reason + '\n' + source.replace('f(', 'g('))
            index = root / (label + '-index')
            run('build', case, index)
            result = run('query', case, index, 1, 60)
            assert result['proven'] + result['check_blocked'] > 0, (label, result)
            intentional.append({'case': label, 'detected': True, 'decision': 'contextual review; no forced extraction',
                                'rationale': reason})
    print(json.dumps({'scope': 'synthetic T=60 probes; no selected product T or agent repair trial',
                      'recommendation': 'ready/check REVIEW only', 'attacks': rows,
                      'intentional_duplicates': intentional,
                      'repair_requirement': 'Feedback must offer contextual review and never demand extraction solely from token equality.'}, indent=2))


if __name__ == '__main__':
    main()
