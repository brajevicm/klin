"""Guarded isolated CLI: invalid raw inputs cannot certify lossy-source reuse."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
binary = Path(sys.argv[1] if len(sys.argv) > 1 else '/tmp/klin490-combined-target/release/klin')
with tempfile.TemporaryDirectory(prefix='klin490-raw-') as tmp:
    paths = []
    for i, content in enumerate([b'\xff', b'\xfe', '\ufffd'.encode()]):
        path = Path(tmp) / f'{i}.ts'
        path.write_bytes(b'const value = "' + content + b'";')
        paths.append(str(path))
    env = dict(os.environ, KLIN_DUP_RAW_PROBE='1', KLIN_DUP_REUSE='1',
               KLIN_DUP_RESEARCH='1', KLIN_DUP_RESEARCH_REPORT='1')
    output = subprocess.check_output([str(binary), *paths], env=env, stderr=subprocess.STDOUT, text=True)
    row = json.loads(next(line.removeprefix('DUP_RESEARCH ') for line in output.splitlines()
                          if line.startswith('DUP_RESEARCH ')))
    assert row['invalid_raw_inputs'] == 2 and row['incomplete']
    assert row['shared_files'] == row['derived_inputs'] == 1
    assert row['reuse_hits'] == 0
    (HERE / 'reuse-raw-guard.json').write_text(json.dumps(row, indent=2) + '\n')
    print('Invalid raw input guard and valid-input derivation passed.')
