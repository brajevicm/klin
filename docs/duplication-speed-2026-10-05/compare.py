"""CLI differential normalization/selection check against the frozen executable."""
import hashlib
import json
import random
import subprocess
import sys
import tempfile
from pathlib import Path


def normalized(binary, path, parameters=()):
    return json.loads(subprocess.check_output([str(binary), 'normalize', str(path), *map(str, parameters)]))


def main():
    reference, candidate, corpus, output = map(Path, sys.argv[1:])
    checked, digest = 0, hashlib.sha256()
    def check(path, params=()):
        nonlocal checked
        before, after = normalized(reference, path, params), normalized(candidate, path, params)
        assert before == after, (str(path), params, [k for k in before if before[k] != after[k]])
        digest.update(json.dumps(before, sort_keys=True).encode())
        checked += 1
    for name, subdir in [('klin', 'klin/src'), ('glaredb', 'glaredb'), ('karakeep', 'karakeep')]:
        count = 0
        for path in sorted((corpus / subdir).rglob('*')):
            if not path.is_file() or path.suffix not in ('.rs', '.ts', '.tsx') or path.name.endswith(('.d.ts', '_test.rs')) or path.name == 'tests.rs':
                continue
            if any(part in ('tests', 'test', '__tests__', 'benches', 'fixtures') for part in path.relative_to(corpus/subdir).parts) or '.test.' in path.name or '.spec.' in path.name:
                continue
            check(path)
            count += 1
        print(name, count, flush=True)
    randomizer = random.Random(490)
    with tempfile.TemporaryDirectory(prefix='klin-dup-diff-') as temp:
        root = Path(temp)
        rust = root/'stress.rs'
        for n in (0, 1, 10, 100, 1000):
            rust.write_text('fn before() { let x = 1; }\n' + ''.join(f'#[cfg ( test )]\n// prelude\n#[test]\nfn test{i}() {{ let x = {i}; }}\nfn kept{i}() {{ let x = {i}; }}\n' for i in range(n)))
            check(rust)
        ts = root/'stress.ts'
        for n in (1, 10, 100):
            ts.write_text(''.join(f'function f{i}() {{ return /*\n*/ x; }}\nfunction g{i}() {{ return x; }}\n' for i in range(n)))
            check(ts)
        for n in (0, 1, 2, 10, 40, 41, 42, 59, 60, 61, 100, 1000):
            for repeated in (False, True):
                values = [0 if repeated else randomizer.randrange(8) for _ in range(n)]
                ts.write_text('function f(x: number) { ' + ' '.join(f'x += {i};' for i in values) + ' return x; }')
                for parameters in ((41, 20, 5), (20, 41, 0), (1, 1, 0), (45, 16, 5), (49, 12, 13)):
                    check(ts, parameters)
    result = {'cases': checked, 'comparison': 'exact JSON token text, rows, safe flags, error/unsafe counts, selected positions',
              'output_sha256': digest.hexdigest(), 'reference_binary_sha256': hashlib.sha256(reference.read_bytes()).hexdigest(),
              'candidate_binary_sha256': hashlib.sha256(candidate.read_bytes()).hexdigest()}
    output.write_text(json.dumps(result, indent=2)+'\n')
    print(checked, 'differential cases passed', flush=True)


if __name__ == '__main__':
    main()
