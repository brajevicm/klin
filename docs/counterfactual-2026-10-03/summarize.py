"""Render raw per-task/arm measurements; no statistical population inference."""
import json
from pathlib import Path
import statistics
import sys

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parent / 'raw'
rows = json.loads((root / 'results.json').read_text())
columns = ['task', 'arm', 'runs', 'cost_before', 'removed_range', 'oracle_calls_median',
           'oracle_seconds_median', 'holdout_seconds_median', 'analysis_seconds_median',
           'harmful_runs', 'inconclusive_observations', 'one_minimal_runs',
           'final_trees', 'repeatable', 'trajectory_bytes', 'feedback_bytes_range']
lines = ['\t'.join(columns)]
for task in sorted({r['task'] for r in rows}):
    task_rows = [r for r in rows if r['task'] == task]
    assert len({r['oracle_id'] for r in task_rows}) == 1
    for arm in 'ABC':
        selected = [r for r in task_rows if r['arm'] == arm]
        if not selected:
            continue
        repeatable = all(len({r['final_tree'] for r in selected
                             if r['order'] == order and r['atomization'] == atoms}) == 1
                         for order in ('forward', 'reverse') for atoms in ('hunk', 'semantic'))
        def span(key):
            values = [r[key] for r in selected]
            return f'{min(values)}..{max(values)}'
        def seconds(evidence):
            return statistics.median(sum(a['seconds'] for observation in r['attempts']
                                         if observation['evidence'] == evidence
                                         for a in observation['attempts']) for r in selected)
        values = [task, arm, len(selected), selected[0]['before_cost'], span('removed_cost'),
                  statistics.median(r['oracle_executions'] for r in selected),
                  round(seconds('oracle'), 6), round(seconds('independent'), 6),
                  round(statistics.median(r['analysis_seconds'] for r in selected), 6),
                  sum(r['harmful'] for r in selected), sum(r['inconclusive'] for r in selected),
                  sum(r['one_minimal'] for r in selected), len({r['final_tree'] for r in selected}),
                  repeatable, selected[0]['trajectory_bytes'], span('feedback_bytes')]
        lines.append('\t'.join(map(str, values)))
(root / 'summary.tsv').write_text('\n'.join(lines) + '\n')
print('\n'.join(lines))
