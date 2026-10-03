#!/usr/bin/env python3
"""Bounded inventory of held configs and static alias sites in existing corpus clones."""
import json
from collections import Counter
import pathlib
import re
import subprocess
import sys


def jsonc(text):
    tokens = re.compile(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*[\s\S]*?\*/')
    clean = tokens.sub(lambda m: m[0] if m[0].startswith('"') else ' ', text)
    clean = re.sub(r',\s*([}\]])', r'\1', clean)
    return json.loads(clean)


def matches(pattern, specifier):
    return re.fullmatch(re.escape(pattern).replace(r'\*', '.*'), specifier) is not None


def inventory(repo):
    files = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', 'HEAD'], text=True).splitlines()
    configs = []
    for file in files:
        if not re.search(r'(^|/)tsconfig[^/]*\.json$', file):
            continue
        try:
            config = jsonc(subprocess.check_output(['git', '-C', str(repo), 'show', 'HEAD:' + file], text=True))
        except (ValueError, UnicodeError):
            configs.append({'file': file, 'unreadable': True})
            continue
        options = config.get('compilerOptions', {})
        configs.append({'file': file, 'paths': options.get('paths', {}),
                        'baseUrl': options.get('baseUrl'), 'extends': config.get('extends'),
                        'references': bool(config.get('references'))})
    sites = []
    for file in files:
        if not file.endswith(('.ts', '.tsx', '.mts', '.cts')):
            continue
        text = subprocess.check_output(['git', '-C', str(repo), 'show', 'HEAD:' + file], text=True, errors='replace')
        for found in re.finditer(r'\b(?:import|export)\s+[^;\n]*?\bfrom\s*["\']([^"\']+)["\']', text):
            specifier = found[1]
            owners = [c for c in configs if str(pathlib.PurePosixPath(c['file']).parent) == '.'
                      or file.startswith(str(pathlib.PurePosixPath(c['file']).parent) + '/')]
            rules = [(c, p, t) for c in owners for p, t in c.get('paths', {}).items() if matches(p, specifier)]
            if not rules:
                continue
            c, p, t = sorted(rules, key=lambda r: ('*' in r[1], -len(r[1].split('*')[0])))[0]
            shape = 'single direct paths mapping'
            if len(owners) != 1:
                shape = 'multiple/nested configs'
            elif c.get('extends'):
                shape = 'local extends' if c['extends'].startswith('.') else 'package extends'
            elif c.get('references'):
                shape = 'project references'
            elif not isinstance(t, list) or len(t) != 1:
                shape = 'multiple paths targets'
            sites.append({'file': file, 'line': text.count('\n', 0, found.start()) + 1,
                          'specifier': specifier, 'config': c['file'], 'shape': shape})
    return {'repository': repo.name.replace('__', '/'),
            'commit': subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip(),
            'configs': configs, 'site_shapes': dict(Counter(site['shape'] for site in sites)),
            'examples': [next(site for site in sites if site['shape'] == shape) for shape in sorted(set(site['shape'] for site in sites))]}


if __name__ == '__main__':
    seen = set()
    rows = []
    for directory in sys.argv[1:]:
        for repo in sorted(pathlib.Path(directory).iterdir()):
            if not (repo / '.git').exists() or repo.name in seen:
                continue
            seen.add(repo.name)
            row = inventory(repo)
            if row['configs']:
                rows.append(row)
    print(json.dumps(rows, indent=2))
