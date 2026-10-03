#!/usr/bin/env python3
"""Root-proof coverage of the survey's direct-config repositories, measured by a klin binary.

Usage: root_proof.py KLIN_BINARY CLONE_DIRECTORY > root-proof-survey.json
"""
import json
import pathlib
import re
import subprocess
import sys

from survey import jsonc

SURVEY = pathlib.Path(__file__).with_name('survey.json')
LAYERS = '{"layering":{"layers":{"app":{"in":".","can_use":[]}}}}'
UNPROVED = 'is a local paths alias whose configuration klin cannot prove'


def supported(pattern):
    if not isinstance(pattern, str) or pattern.startswith('/'):
        return False
    if re.fullmatch(r'(?:[^*?]+/)?\*\*/\*\.tsx?', pattern):
        return True
    path = '' if pattern == '**/*' else pattern.removesuffix('/**/*')
    return '*' not in path and '?' not in path


def shapes(root):
    found = {'include': [], 'exclude': []}
    for config in root.rglob('tsconfig*.json'):
        if 'node_modules' in config.parts:
            continue
        try:
            value = jsonc(config.read_text())
        except (ValueError, UnicodeError):
            continue
        for key in found:
            found[key] += [p for p in value.get(key, []) if not supported(p)]
    return {f'unsupported_{key}': sorted(set(map(str, found[key]))) for key in found}


def clone(repository, commit, directory):
    source = directory / repository.replace('/', '__')
    if not source.exists():
        subprocess.run(['git', 'init', '-q', str(source)], check=True)
        subprocess.run(['git', '-C', str(source), 'fetch', '-q', '--depth', '1',
                        f'https://github.com/{repository}.git', commit], check=True)
    tree = directory / (source.name + '-tree')
    subprocess.run(['rm', '-rf', str(tree)], check=True)
    tree.mkdir()
    archive = subprocess.run(['git', '-C', str(source), 'archive', commit], check=True, capture_output=True).stdout
    subprocess.run(['tar', '-x', '-C', str(tree)], input=archive, check=True)
    for command in (['init', '-q', '-b', 'main'], ['commit', '-q', '--allow-empty', '-m', 'base'],
                    ['checkout', '-q', '-b', 'run']):
        subprocess.run(['git', '-C', str(tree), *command], check=True)
    (tree / 'klin.json').write_text(LAYERS)
    return tree


def measure(klin, row, directory):
    tree = clone(row['repository'], row['commit'], directory)
    run = subprocess.run([klin, 'gate', '--gate', 'layering', '--json'], cwd=tree, capture_output=True, text=True)
    result = json.loads(run.stdout)
    holes = [f for f in result['findings'] if f['outcome'] == 'unresolved' and 'local paths alias' in f['text']]
    unproved = sum(UNPROVED in f['text'] for f in holes)
    recognized = row['site_shapes'].get('single direct paths mapping', 0)
    return {'repository': row['repository'], 'commit': row['commit'], 'exit': result['exit'],
            'recognized_direct_sites': recognized,
            'graph_dependencies': result['gates'][0]['graph']['dependencies'],
            'unproved_root_or_config_holes': unproved,
            'other_alias_holes': len(holes) - unproved,
            **shapes(tree)}


if __name__ == '__main__':
    klin, directory = sys.argv[1], pathlib.Path(sys.argv[2])
    rows = [r for r in json.loads(SURVEY.read_text()) if r['site_shapes'].get('single direct paths mapping')]
    print(json.dumps([measure(klin, row, directory) for row in rows], indent=2))
