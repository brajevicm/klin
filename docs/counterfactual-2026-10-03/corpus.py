"""Constructed calibration tasks, not sampled autonomous agent trajectories.

Before/after segments are the independent replay source. Trajectory records carry
only turn/tree/group identities; no prompts or source text. Contracts are frozen
before search and never used to select reductions.
"""


def segment(path, before, after, turn):
    return dict(path=path, before=before, after=after, turn=turn)


def task(name, repo, shape, segments, test, independent, intent, **extra):
    return dict(name=name, repo=repo, shape=shape, segments=segments,
                oracle=test, independent=independent, intent=intent, **extra)


def cases():
    py = 'ledger.py'
    js = 'router.mjs'
    ledger = 'from ledger import *\n'
    router = "import * as r from './router.mjs';\nimport assert from 'node:assert/strict';\n"
    # Unchanged separators force distinct diff hunks without altering behavior.
    gap = '\n' + '\n'.join('# separator ' + str(i) for i in range(8)) + '\n'
    jgap = gap.replace('#', '//')
    return [
        task('bug', 'ledger', 'bug repair', [
            segment(py, 'def amount(n):\n    return n\n',
                    'def amount(n):\n    return abs(n)\n', 2),
            segment(py, gap, gap, 0),
            segment(py, '', 'def debug(n):\n    return str(n)\n', 1)],
            ledger + 'assert amount(-3) == 3\nassert amount(4) == 4\n',
            ledger + 'assert amount(-7) == 7\nassert amount(0) == 0\n',
            'Repair negative amounts without changing positive amounts.', preexisting=True),
        task('weak-feature', 'ledger', 'feature', [
            segment(py, 'def normalize(s):\n    return s\n',
                    'def normalize(s):\n    return s.lower()\n', 1)],
            ledger + "assert isinstance(normalize('ABC'), str)\n",
            ledger + "assert normalize('AbC') == 'abc'\n",
            'Add case normalization. Candidate-authored assertion only checks type.',
            candidate_authored=True),
        task('wrong-expectation', 'ledger', 'feature', [
            segment(py, 'def amount(n):\n    return n * 2\n',
                    'def amount(n):\n    return n * 3\n', 1)],
            ledger + 'assert amount(2) == 6\n',
            ledger + 'assert amount(2) == 4\n',
            'Double amounts; candidate test encodes its own incorrect tripling behavior.',
            candidate_authored=True, contract=True),
        task('performance', 'ledger', 'refactor', [
            segment(py, 'def lookup(xs, key):\n    return next(v for k, v in xs if k == key)\n',
                    'def lookup(xs, key):\n    return xs[key]\n', 1)],
            ledger + "class Both(dict):\n    def __iter__(self): return iter(self.items())\nassert lookup(Both({0: 2, 1: 3}), 1) == 3\n",
            ledger + "class Indexed:\n    def __getitem__(self, k): return 3\n    def __iter__(self): raise AssertionError('must not scan')\nassert lookup(Indexed(), 1) == 3\n",
            'Use indexed input without scanning; functional examples do not observe operation count.',
            preexisting=True, nonfunctional=True),
        task('architecture', 'ledger', 'architecture', [
            segment(py, 'def price(n):\n    return n * 2\n',
                    'def price(n, provider=lambda n: n * 2):\n    return provider(n)\n', 1)],
            ledger + 'assert price(3) == 6\n',
            ledger + 'assert price(3, lambda n: n * 4) == 12\n',
            'Retain the provider boundary for future pricing extensions.', preexisting=True),
        task('ordering', 'ledger', 'feature', [
            segment(py, 'def a(n):\n    return 0\n', 'def a(n):\n    return n * 2\n', 1),
            segment(py, gap, gap, 0),
            segment(py, 'def b(n):\n    return 0\n', 'def b(n):\n    return n * 2\n', 2),
            segment(py, 'def total(n):\n    return max(a(n), b(n))\n',
                    'def total(n):\n    return max(a(n), b(n))\n', 0)],
            ledger + 'assert total(3) == 6\n', ledger + 'assert total(7) == 14\n',
            'Double nonnegative totals; two interchangeable implementations challenge order.',
            contract=True),
        task('flaky', 'ledger', 'bug repair', [
            segment(py, 'def amount(n):\n    return n\n',
                    'def amount(n):\n    return abs(n)\n', 1),
            segment(py, '', 'def unused():\n    return 1\n', 2)],
            ledger + "import os\nfrom pathlib import Path\np = Path(os.environ['PROBE_STATE'])\nn = int(p.read_text()) if p.exists() else 0\np.write_text(str(n + 1))\nassert n % 2 == 0, 'injected alternating instability'\nassert amount(-3) == 3\n",
            ledger + 'assert amount(-9) == 9\n',
            'Repair negative amounts; injected alternation is an inconclusive oracle.', preexisting=True),
        task('api', 'router', 'API extension', [
            segment(js, 'export const route = s => s;\n', 'export const route = s => s;\n', 0),
            segment(js, '', 'export const externalRoute = s => s.toLowerCase();\n', 1)],
            router + "assert.equal(r.route('a'), 'a');\n",
            router + "assert.equal(r.externalRoute('AB'), 'ab');\n",
            'Expose externalRoute to consumers absent from repository tests.', preexisting=True),
        task('integration', 'router', 'dependency/integration', [
            segment(js, "export const route = s => s;\n", "export const route = s => s;\n", 0),
            segment('compat.mjs', '', 'export const legacyRoute = s => s;\n', 1),
            segment('vendor/generated.mjs', '', 'export const generated = 1;\n', 1)],
            router + "assert.equal(r.route('a'), 'a');\n",
            router + "const old = await import('./compat.mjs');\nassert.equal(old.legacyRoute('a'), 'a');\n",
            'Keep the intentionally duplicated legacy shim; generated/vendor edits are out of search.',
            preexisting=True, excluded=['vendor/']),
        task('dynamic', 'router', 'feature', [
            segment(js, "export const route = s => s;\n", "export const route = s => s;\n", 0),
            segment('plugin.mjs', '', "export const activate = registry => registry.set('extra', s => s.toUpperCase());\n", 1)],
            router + "assert.equal(r.route('a'), 'a');\n",
            router + "const {activate} = await import('./plugin.mjs');\nconst registry = new Map();\nactivate(registry);\nassert.equal(registry.get('extra')('a'), 'A');\n",
            'Retain a dynamically loaded plugin; static tests do not reach it.', preexisting=True),
        task('paired-history', 'router', 'refactor + feature', [
            segment(js, 'const send = s => s;\n', 'const deliver = s => s;\n', 1),
            segment(js, jgap, jgap, 0),
            segment(js, 'export const route = s => send(s);\n',
                    'export const route = s => deliver(s);\n', 1),
            segment(js, jgap, jgap, 0),
            segment(js, 'export const normalize = s => s;\n',
                    'export const normalize = s => s.toLowerCase();\n', 2)],
            router + "assert.equal(r.route('a'), 'a');\nassert.equal(r.normalize('AB'), 'ab');\n",
            router + "assert.equal(r.route('z'), 'z');\nassert.equal(r.normalize('XY'), 'xy');\n",
            'Add normalization. Exploratory rename may be reverted as a coordinated pair.',
            preexisting=True),
        task('boundary', 'router', 'refactor', [
            segment(js, 'export const route = s => s;\n',
                    'export const route = s => adapt(s);\n', 1),
            segment(js, jgap, jgap, 0),
            segment(js, '', 'const adapt = s => s;\n', 1)],
            router + "assert.equal(r.route('a'), 'a');\n",
            router + "assert.equal(r.route('z'), 'z');\n",
            'Keep a named adaptation boundary for maintainers even though current behavior is identity.',
            preexisting=True, design_review=True,
            transform={'router.mjs': 'export const route = s => s;\n' + jgap})
    ]
