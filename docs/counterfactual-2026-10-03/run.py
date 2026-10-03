"""Disposable #356 experiment. No imports from klin and no repository mutations."""
import argparse
from collections import defaultdict
import difflib
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from corpus import cases

LIMIT = 2.0
OUTPUT_LIMIT = 16384


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def command(argv, cwd, state):
    """Bound the command group and captured output, including inherited pipes."""
    start = time.monotonic()
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', PROBE_STATE=str(state))
    proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, start_new_session=True)
    captured = bytearray()
    reason = None
    with selectors.DefaultSelector() as selector:
        selector.register(proc.stdout, selectors.EVENT_READ)
        while selector.get_map():
            if time.monotonic() - start >= LIMIT:
                reason = 'timeout'
                break
            for key, _ in selector.select(0.02):
                chunk = os.read(key.fileobj.fileno(), 4096)
                if not chunk:
                    selector.unregister(key.fileobj)
                else:
                    captured.extend(chunk)
                    if len(captured) > OUTPUT_LIMIT:
                        reason = 'output-bound'
                        break
            if reason:
                break
    if reason is None:
        try:
            proc.wait(timeout=0.05)
        except subprocess.TimeoutExpired:
            reason = 'pipe-closed-before-exit'
    proc.poll()
    # Kill same-group descendants even if the immediate process already exited.
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    code = proc.wait()
    proc.stdout.close()
    if code < 0 and reason is None:
        reason = 'abnormal-termination'
    return dict(code=code, reason=reason, seconds=time.monotonic() - start,
                output=bytes(captured[:OUTPUT_LIMIT]).decode(errors='replace'))


def tree(case, retained):
    files = defaultdict(str)
    for i, atom in enumerate(case['segments']):
        files[atom['path']] += atom['after'] if i in retained else atom['before']
    return {p: s for p, s in sorted(files.items()) if s}


def cost(base, files):
    count = 0
    for path in base.keys() | files.keys():
        for line in difflib.ndiff(base.get(path, '').splitlines(), files.get(path, '').splitlines()):
            count += line.startswith(('+ ', '- '))
    return count


def run_evidence(case, files, text, state, repeats):
    attempts = []
    suffix = '.py' if case['repo'] == 'ledger' else '.mjs'
    with tempfile.TemporaryDirectory(prefix='counterfactual356-') as scratch:
        root = Path(scratch)
        for path, source in files.items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(source)
        script = root / ('oracle' + suffix)
        script.write_text(text)
        argv = [sys.executable if suffix == '.py' else shutil.which('node'), str(script)]
        initial = {str(p.relative_to(root)): p.read_bytes().hex()
                   for p in root.rglob('*') if p.is_file()}
        for _ in range(repeats):
            observation = command(argv, root, state)
            after = {str(p.relative_to(root)): p.read_bytes().hex()
                     for p in root.rglob('*') if p.is_file()}
            if after != initial:
                observation['reason'] = 'tree-drift'
            attempts.append(observation)
    outcomes = [a['code'] == 0 for a in attempts]
    incomplete = any(a['reason'] for a in attempts) or len(set(outcomes)) > 1
    return dict(state='inconclusive' if incomplete else ('pass' if all(outcomes) else 'fail'),
                attempts=attempts, scratch_cleaned=not root.exists())


def groups(case, atomization):
    eligible = [i for i, a in enumerate(case['segments']) if a['before'] != a['after']
                and not any(a['path'].startswith(p) for p in case.get('excluded', []))]
    by_file = defaultdict(list)
    by_turn = defaultdict(list)
    for i in eligible:
        a = case['segments'][i]
        by_file[a['path']].append(i)
        by_turn[a['turn']].append(i)
    # Hunk spans from the final diff, then project onto immutable source segments.
    hunks = []
    for path, ids in by_file.items():
        atoms = [(i, a) for i, a in enumerate(case['segments']) if a['path'] == path]
        before = ''.join(a['before'] for _, a in atoms).splitlines()
        after = ''.join(a['after'] for _, a in atoms).splitlines()
        ranges = {}
        position = 0
        for i, a in atoms:
            end = position + len(a['after'].splitlines())
            ranges[i] = (position, end)
            position = end
        for hunk in difflib.SequenceMatcher(None, before, after, autojunk=False).get_grouped_opcodes(3):
            selected = set()
            for tag, _, _, start, end in hunk:
                if tag == 'equal':
                    continue
                for i in ids:
                    low, high = ranges[i]
                    if low < end and high > start or start == end and low <= start <= high:
                        selected.add(i)
            if selected:
                hunks.append(sorted(selected))
    fine = [[i] for i in eligible] if atomization == 'semantic' else []
    return eligible, [list(by_file.values()), hunks, fine], list(by_turn.values())


def trajectory(case):
    retained = set()
    history = []
    for turn in sorted({a['turn'] for a in case['segments']} - {0}):
        before = digest(tree(case, retained))
        ids = [i for i, a in enumerate(case['segments']) if a['turn'] == turn]
        retained.update(ids)
        history.append(dict(turn=turn, before=before, after=digest(tree(case, retained)), groups=ids))
    return history


def experiment(case, arm, ordering, atomization, repetition, output, repeats):
    start = time.monotonic()
    eligible, patch_groups, turn_groups = groups(case, atomization)
    retained = {i for i, a in enumerate(case['segments']) if a['before'] != a['after']}
    base = tree(case, set())
    final = tree(case, retained)
    external = 0.0
    attempts = []
    considered = 0
    state = output / ('state-' + case['name'])
    state.unlink(missing_ok=True)

    def evaluate(files, label, evidence='oracle'):
        nonlocal external
        observation = run_evidence(case, files, case[evidence], state, repeats)
        external += sum(a['seconds'] for a in observation['attempts'])
        attempts.append(dict(candidate=label, tree=digest(files), evidence=evidence, **observation))
        return observation['state']

    initial = evaluate(final, 'initial')
    levels = ([turn_groups] if arm == 'B' else []) + patch_groups
    if ordering == 'reverse':
        levels = [list(reversed(level)) for level in levels]
    search_groups = [group for level in levels for group in level]
    transformed = False
    if initial == 'pass':
        if arm == 'C' and case.get('transform'):
            considered += 1
            candidate = dict(final)
            candidate.update(case['transform'])
            if evaluate(candidate, 'collapse-identity-boundary') == 'pass':
                final = candidate
                transformed = True
        elif arm != 'C':
            changed = True
            while changed:
                changed = False
                for group in search_groups:
                    remove = retained.intersection(group)
                    if not remove:
                        continue
                    considered += 1
                    candidate = tree(case, retained - remove)
                    if evaluate(candidate, sorted(remove)) == 'pass':
                        retained -= remove
                        final = candidate
                        changed = True
    # Audit one-minimality only over the declared atoms on the final tree.
    audit = []
    if initial == 'pass' and arm != 'C':
        for i in eligible:
            if i in retained:
                audit.append(evaluate(tree(case, retained - {i}), ['audit', i]))
    one_minimal = bool(initial == 'pass' and arm != 'C' and all(s == 'fail' for s in audit))
    original = tree(case, {i for i, a in enumerate(case['segments']) if a['before'] != a['after']})
    independent_before = evaluate(original, 'original-independent-holdout', 'independent')
    independent = evaluate(final, 'independent-holdout', 'independent')
    design_lost = case.get('design_review', False) and final != tree(case, {i for i, a in enumerate(case['segments']) if a['before'] != a['after']})
    # Researcher rubric is separate from executable behavior checks; not a human vote.
    harmful = initial == 'pass' and final != original and ((independent_before == 'pass' and independent == 'fail') or design_lost)
    before_cost = cost(base, tree(case, {i for i, a in enumerate(case['segments']) if a['before'] != a['after']}))
    after_cost = cost(base, final)
    history = trajectory(case)
    history_bytes = len(json.dumps(history, separators=(',', ':')).encode())
    feedback = ''
    if before_cost > after_cost:
        feedback = (f"REVIEW {case['name']}: a candidate removed {before_cost - after_cost} edit lines "
                    f"under frozen oracle {digest(case['oracle'])[:12]}. Preserve requested intent: "
                    f"{case['intent']} Independent holdout: {independent}. "
                    'Inspect one candidate; do not edit merely to clear this observation.')
    row = dict(task=case['name'], repository=case['repo'], shape=case['shape'], arm=arm,
               order=ordering, atomization=atomization, repetition=repetition,
               oracle_id=digest(dict(text=case['oracle'], runtime=RUNTIMES[case['repo']],
                                    repeats=repeats, timeout=LIMIT, output=OUTPUT_LIMIT)),
               initial=initial, groups_considered=considered, audit_groups=len(audit),
               declared_search_groups=len(search_groups) if arm != 'C' else int(bool(case.get('transform'))),
               oracle_executions=sum(len(a['attempts']) for a in attempts if a['evidence'] == 'oracle'),
               independent_executions=sum(len(a['attempts']) for a in attempts if a['evidence'] == 'independent'),
               external_seconds=external, analysis_seconds=time.monotonic() - start - external,
               klin_native_seconds=0, before_cost=before_cost, after_cost=after_cost,
               removed_cost=before_cost - after_cost,
               removable_share=(before_cost - after_cost) / before_cost if before_cost else 0,
               one_minimal=one_minimal, independent=independent, independent_before=independent_before, harmful=harmful,
               design_rubric_violation=design_lost, human_review='not-run',
               inconclusive=sum(a['state'] == 'inconclusive' for a in attempts),
               final_tree=digest(final), retained=sorted(retained) if not transformed else None,
               trajectory_bytes=history_bytes, trajectory_events=len(history),
               surfaced_candidates=int(bool(feedback)), feedback_bytes=len(feedback.encode()),
               agent_turns=None, agent_understanding='not-run', repair_quality='not-run',
               attempts=attempts, transformed=transformed)
    return row


def security_probe(output):
    with tempfile.TemporaryDirectory(prefix='security356-') as work:
        root = Path(work)
        # A dirty caller is outside the disposable candidate; no Git reset/clean.
        dirty = root / 'uncommitted.txt'
        dirty.write_text('uncommitted caller work')
        state = root / 'state'
        marker = root / 'survived'
        case = dict(repo='ledger')
        files = {'ledger.py': 'value = 1\n'}
        child = f"import time; from pathlib import Path; time.sleep(5); Path({str(marker)!r}).write_text('alive')"
        scripts = {
            'timeout': 'import time; time.sleep(10)',
            'output': "print('x' * 100000)",
            'drift': "from pathlib import Path; Path('ledger.py').write_text('value = 2')",
            'failure': "raise AssertionError('known rejected candidate')",
            'descendant_inherited_pipe': f"import subprocess, sys; subprocess.Popen([sys.executable, '-c', {child!r}])",
            'descendant_closed_pipe': f"import subprocess, sys; subprocess.Popen([sys.executable, '-c', {child!r}], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)",
        }
        observations = {name: run_evidence(case, files, script, state, 1)
                        for name, script in scripts.items()}
        time.sleep(5.1)
        observations['descendant_survived'] = marker.exists()
        observations['cleanup_complete'] = all(o['scratch_cleaned'] for o in observations.values() if isinstance(o, dict))
        observations['dirty_caller_preserved'] = dirty.read_text() == 'uncommitted caller work'
    output.mkdir(parents=True, exist_ok=True)
    (output / 'security.json').write_text(json.dumps(observations, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--task', default='all')
    parser.add_argument('--security', action='store_true')
    parser.add_argument('--repeat', type=int, default=2)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.security:
        security_probe(args.output)
        sys.exit(0)
    if args.repeat < 1:
        parser.error('--repeat must be positive')
    RUNTIMES = {'ledger': subprocess.check_output([sys.executable, '--version'], text=True).strip(),
                'router': subprocess.check_output(['node', '--version'], text=True).strip()}
    args.output.mkdir(parents=True, exist_ok=True)
    selected = [c for c in cases() if args.task in ('all', c['name'])]
    if not selected:
        parser.error('unknown task')
    rows = []
    for case in selected:
        for arm in ('A', 'B', 'C'):
            for order in ('forward', 'reverse'):
                for atomization in ('hunk', 'semantic'):
                    for repetition in range(args.repeat):
                        row = experiment(case, arm, order, atomization, repetition,
                                         args.output, 3 if case['name'] == 'flaky' else 1)
                        rows.append(row)
        print(case['name'], 'completed', flush=True)
    (args.output / 'results.json').write_text('[\n' + ',\n'.join(json.dumps(row, separators=(',', ':')) for row in rows) + '\n]\n')
    (args.output / 'trajectory.json').write_text(json.dumps({c['name']: trajectory(c) for c in selected}, separators=(',', ':')) + '\n')
    (args.output / 'environment.json').write_text(json.dumps(dict(
        runtimes=RUNTIMES, platform=sys.platform, python=sys.executable,
        node=shutil.which('node'), corpus_id=digest(cases()),
        timeout=LIMIT, output_bound=OUTPUT_LIMIT), indent=2) + '\n')
    for path in args.output.glob('state-*'):
        path.unlink()
