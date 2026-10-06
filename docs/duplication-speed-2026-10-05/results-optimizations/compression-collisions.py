"""Candidate width changes collisions, not text-confirmed output."""
import json
import pathlib
import subprocess
import tempfile

BIN = '/tmp/klin490-opt-reference/dup-speed'


def run(*args):
    return json.loads(subprocess.check_output([BIN, *map(str, args)]))


with tempfile.TemporaryDirectory(prefix='klin490-compression-') as tmp:
    root = pathlib.Path(tmp)
    source = root / 'source'
    source.mkdir()
    (source / 'a.ts').write_text('collision_47066;')
    (source / 'b.ts').write_text('collision_48240;')
    results = []
    for bits in (32, 48, 64):
        index = root / str(bits)
        run('build', source, index, 1, 1, 64, 0, bits)
        query = run('query', source, index, 1, 1)
        assert query['check_blocked'] == query['true_regions'] == 1, query
        assert query['check_false'] == query['check_missed'] == 0, query
        if bits == 32:
            assert query['false_regions'] > 0, query
        results.append(dict(bits=bits, **query))
    print(json.dumps(results, indent=2))
