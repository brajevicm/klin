#!/usr/bin/env python3
"""Replay both previously green alias attacks against the shipped graph, through the CLI."""
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

repo = pathlib.Path(__file__).resolve().parents[2]
binary = str(pathlib.Path(sys.argv[1]).resolve())
fixtures = [
    ('361', repo / 'docs/appeasement-audit-2026-10-02/fixtures/layering-ts'),
    ('355', repo / 'docs/design-conformance-2026-10-02/fixtures/payments-ts'),
]
rows = []
for issue, fixture in fixtures:
    with tempfile.TemporaryDirectory(prefix='klin445-') as temporary:
        root = pathlib.Path(temporary)
        shutil.copytree(fixture / 'base', root, dirs_exist_ok=True)
        if issue == '355':
            (root / 'klin.json').write_text(json.dumps({'build': [], 'layering': {
                'in': 'src', 'acyclic': True,
                'layers': {'db': {'in': 'src/db', 'can_use': []},
                           'ui': {'in': 'src/ui', 'can_use': ['db']}}}}))
        for args in [['init', '-q'], ['config', 'user.email', 'test@example.com'],
                     ['config', 'user.name', 'test'], ['add', '.'], ['commit', '-qm', 'base'],
                     ['branch', '-M', 'main']]:
            subprocess.run(['git', *args], cwd=root, check=True)
        subprocess.run([binary, 'radius'], cwd=root, check=True, capture_output=True)
        shutil.copytree(fixture / ('alias' if issue == '361' else 'attack-alias'), root, dirs_exist_ok=True)
        for name, args, payload in [
            ('CI', ['gate', '--changed', '--gate', 'layering', '--json'], None),
            ('Stop', ['gate', '--hook', '--changed', '--gate', 'layering'],
             '{"hook_event_name":"Stop","stop_hook_active":false}')]:
            run = subprocess.run([binary, *args], cwd=root, input=payload, text=True, capture_output=True)
            assert run.returncode == (1 if name == 'CI' else 2), (issue, name, run.stdout, run.stderr)
            rows.append({'issue': issue, 'caller': name, 'exit': run.returncode,
                         'stdout': json.loads(run.stdout) if name == 'CI' else run.stdout,
                         'stderr': run.stderr})
print(json.dumps(rows, indent=2))
