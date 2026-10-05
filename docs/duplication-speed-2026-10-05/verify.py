"""Small exhaustive text oracle and parser-risk probes; uses the real CLI."""
import json
import os
import random
import subprocess
import sys
import tempfile
from pathlib import Path

BIN = Path(os.environ.get('DUP_SPEED', Path(__file__).parent / 'proto/target/release/dup-speed'))
REPORT = []


# ponytail: full prototype checks can expand O(pairs); use bounded fixtures,
# require cohort reporting and work budgets before shipping a full check.
def run(*args):
    return json.loads(subprocess.check_output([str(BIN), *map(str, args)], text=True))


def normalized(path):
    return run('normalize', path)


def matches(a, b, threshold, self_pair=False):
    """Enumerate maximal exact eligible runs on every diagonal, without hashes."""
    found = []
    for diagonal in range(-len(a['tokens']) + 1, len(b['tokens'])):
        if self_pair and diagonal <= 0:
            continue
        start = max(0, -diagonal)
        end = min(len(a['tokens']), len(b['tokens']) - diagonal)
        first = start
        for i in range(start, end + 1):
            equal = i < end and a['safe'][i] and b['safe'][i + diagonal] and a['tokens'][i] == b['tokens'][i + diagonal]
            if equal:
                continue
            if i - first >= threshold:
                found.append((diagonal, first, i))
            first = i + 1
    return found


def pair(root, label, left, right, threshold=60, extensions=('ts', 'ts'), params=()):
    source = root / label
    source.mkdir()
    a, b = [source / (name + '.' + ext) for name, ext in zip(('a', 'b'), extensions)]
    a.write_text(left)
    b.write_text(right)
    index = root / (label + '-index')
    build = run('build', source, index, *params)
    na, nb = normalized(a), normalized(b)
    expected = len(matches(na, nb, threshold)) + len(matches(na, na, threshold, True))
    result = run('query', source, index, 1, threshold)
    assert result['true_regions'] == expected, (label, expected, result)
    assert result['false_regions'] == result['check_false'] == 0, (label, result)
    assert result['check_missed'] == 0, (label, result)
    REPORT.append({'case': label, 'threshold': threshold, 'oracle_regions': expected,
                   'stop_proven': result['proven'], 'check_blocked': result['check_blocked'],
                   'incomplete': result['incomplete'], 'unsafe_units': na['unsafe_units'] + nb['unsafe_units']})
    return na, nb, result


def body(n=40):
    return ' '.join(f'x += {i};' for i in range(n))


def main():
    with tempfile.TemporaryDirectory(prefix='klin-dup-oracle-') as directory:
        root = Path(directory)
        collision = root / 'key-collision'
        collision.mkdir()
        (collision / 'a.ts').write_text('collision_47066;')
        (collision / 'b.ts').write_text('collision_48240;')
        # Distinct identifiers share a 32-bit key. Only the semicolon is equal.
        index = root / 'key-collision-index'
        run('build', collision, index, 1, 1, 64, 0, 32)
        result = run('query', collision, index, 1, 1)
        assert result['false_regions'] > 0, result
        assert result['check_blocked'] == 1 and result['check_false'] == 0, result
        assert result['true_regions'] == 1 and result['check_missed'] == 0, result
        REPORT.append({'case': '32-bit-collision-text-confirmation', 'threshold': 1,
                       'hash_false_regions': result['false_regions'],
                       'verified_text_regions': result['check_blocked']})
        for n in (11, 12, 13, 19, 20, 21, 29, 30, 31, 40):
            for threshold in (60, 80, 100, 150):
                source = f'function f(x: number) {{ {body(n)} return x; }}'
                pair(root, f'boundary-{n}-{threshold}', source, source, threshold)
        source = f'function f(x: number) {{ {body()} return x; }}'
        pair(root, 'renamed-body', source, source.replace('f(', 'g('))
        pair(root, 'edited-middle', source, source.replace('x += 19;', 'x -= 19;'))
        pair(root, 'comments-layout', source, source.replace(';', '; /* note */\n'))
        pair(root, 'ts-tsx', source, source, extensions=('ts', 'tsx'))
        pair(root, 'rust-ts', 'fn f(mut x: u32) -> u32 { ' + body() + ' x }', source, extensions=('rs', 'ts'))
        for label, unsafe in (
            ('error', f'function f(x: number) {{ {body()} var await = 1; return await; }}'),
            ('missing', f'function f(x: number) {{ {body()} return x;'),
            ('silent-comment', f'function f(x: number) {{ {body()} return /*\n*/ x; }}'),
            ('silent-comment-cr', f'function f(x: number) {{ {body()} return /*\r*/ x; }}'),
            ('silent-comment-ls', f'function f(x: number) {{ {body()} return /*\u2028*/ x; }}'),
        ):
            a, b, result = pair(root, label, unsafe, unsafe)
            assert a['unsafe_units'] > 0 and result['proven'] == result['check_blocked'] == 0
        for label, explicit, automatic in (
            ('class-method', 'class C { f() { return 1; }; }', 'class C { f() { return 1; } }'),
            ('export-signature', 'export function f(): void;', 'export function f(): void'),
            ('class-member', 'class C { x: number = 1; y: number = 2; }', 'class C { x: number = 1\ny: number = 2\n}'),
            ('interface-member', 'interface C { x: number; y: number; }', 'interface C { x: number\ny: number\n}'),
            ('for-separators', 'function f() { for (;;) { break; } }', 'function f() { for (;;) { break\n} }'),
            ('export-wrap', 'export const x = 1;', 'export const x = 1'),
        ):
            a, b, _ = pair(root, label, explicit, automatic)
            assert a['tokens'] == b['tokens'], label
        clean = 'fn f(mut x: u32) -> u32 { ' + body() + ' x }'
        test = '#[ cfg ( test ) ]\n// annotation\n#[allow(dead_code)]\nmod t { ' + clean + ' }'
        a, b, result = pair(root, 'rust-test-ranges', clean, test, extensions=('rs', 'rs'))
        assert result['true_regions'] == 0 and not b['tokens']
        # Required pinned-parser probes: parsing succeeds; token text stays as written.
        for label, probe in (
            ('conditional-arrow', 'const f = flag ? (x: number) => x + 1 : (x: number) => x - 1;'),
            ('type-assertion', 'const x = <number>value;'),
            ('yield-identifier', 'function f() { var yield = 1; return yield; }'),
            ('regex-division', 'const a = /x/.test(s); const b = x / y / z;'),
            ('let-identifier', 'function f() { var let = 1; return let; }'),
            ('generic-call', 'const a = f<T>(x); const b = (f < T) > x;'),
        ):
            path = root / (label + '.ts')
            path.write_text(probe)
            parsed = normalized(path)
            assert not parsed['error'], (label, parsed)
            REPORT.append({'case': label, 'error': parsed['error'], 'unsafe_units': parsed['unsafe_units'],
                           'tokens': [bytes.fromhex(t)[1:].decode() for t in parsed['tokens']]})
        for seed in range(30):
            rng = random.Random(seed)
            for repetitive in (False, True):
                values = [rng.randrange(4 if repetitive else 1000000) for _ in range(250)]
                path = root / f'positions-{seed}-{repetitive}.ts'
                path.write_text('function f() { ' + ' '.join(f'f({v});' for v in values) + ' }')
                parsed = normalized(path)
                positions = parsed['positions']
                assert positions == sorted(set(positions)), (seed, repetitive)
                assert all(b - a <= 20 for a, b in zip(positions, positions[1:])), (seed, repetitive)
                assert positions[0] < 20
                assert positions[-1] >= len(parsed['tokens']) - 41 - 20 + 1
        REPORT.append({'case': 'production-minimizer-properties', 'streams': 60, 'k': 41, 'w': 20, 't': 5})
        # Decoys precede the real copy; caps may find some hits but cannot clear uncertainty.
        for multiplicity in (63, 64, 65):
            source_dir = root / f'cap-{multiplicity}'
            source_dir.mkdir()
            target = f'function changed(x: number) {{ {body(30)} return x; }}'
            (source_dir / 'a.ts').write_text(target)
            for i in range(multiplicity):
                decoy = f'function decoy{i}(x: number) {{ {body(12)} return {10000+i}; }}'
                (source_dir / f'b{i:04}.ts').write_text(decoy)
            (source_dir / 'z.ts').write_text(target)
            index = root / f'cap-{multiplicity}-index'
            run('build', source_dir, index)
            result = run('query', source_dir, index, 1, 100)
            assert result['check_missed'] == 0, result
            if result['capped_hits']:
                assert result['incomplete'], result
            REPORT.append({'case': f'cap-boundary-{multiplicity}', **result})
        source_dir = root / 'capped-decoys'
        source_dir.mkdir()
        target = 'function f() { ' + body(12) + ' x++; return x; }'
        (source_dir / 'a.ts').write_text(target)
        target_stream = normalized(source_dir / 'a.ts')
        assert len(target_stream['tokens']) == 60 and len(target_stream['positions']) == 1
        selected = target_stream['positions'][0]
        gram = target_stream['tokens'][selected:selected+41]
        decoy_path = source_dir / 'b0000.ts'
        accepted = 0
        for attempt in range(5000):
            decoy = target.replace('function f()', f'function decoy{attempt}()').replace('return x;', f'return value{attempt};')
            decoy_path.write_text(decoy)
            candidate = normalized(decoy_path)
            if candidate['positions'] == [selected] and candidate['tokens'][selected:selected+41] == gram:
                decoy_path.rename(source_dir / f'b{accepted:04}.ts')
                accepted += 1
                if accepted == 64:
                    break
                decoy_path = source_dir / f'b{accepted:04}.ts'
        assert accepted == 64
        (source_dir / 'z.ts').write_text(target)
        index = root / 'capped-decoys-index'
        run('build', source_dir, index)
        result = run('query', source_dir, index, 1, 60)
        assert result['capped_hits'] == 1 and result['unanchored'] == 0, result
        assert result['proven'] == 0 and result['incomplete'], result
        assert result['check_blocked'] >= 1 and result['check_missed'] == 0, result
        REPORT.append({'case': '64-short-decoys-before-true-partner', **result})
        # Stop must function with the validation token chain absent.
        (index / 'chain.bin').unlink()
        run('query', source_dir, index, 1, 100, 'spread', 'stop')
    output = json.dumps({'oracle': 'exhaustive normalized token text; no fingerprints', 'cases': REPORT}, indent=2)
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(output + '\n')
    print(f'{len(REPORT)} oracle/probe cases passed')


if __name__ == '__main__':
    main()
