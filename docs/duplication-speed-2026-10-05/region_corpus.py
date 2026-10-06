"""Normalize once; compare complete exact corpus maps and diagnostic costs."""
import gc
import hashlib
import json
import resource
import runpy
import subprocess
import sys
import time
from pathlib import Path

import families
import region_cohorts
import suffix_cohorts

eligible = runpy.run_path(str(Path(__file__).parent / 'results-optimizations/compression.py'))['eligible']


def main():
    root = Path(sys.argv[1]).resolve()
    records = []
    started = time.perf_counter()
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if path.is_file() and eligible(relative):
            data = json.loads(subprocess.check_output([str(families.verify.BIN), 'normalize', str(relative)], cwd=root))
            records.append((str(relative), data))
    normalization_ms = (time.perf_counter() - started) * 1000
    rows = []
    maps = []
    for name, matcher in [('seed', region_cohorts.indexed), ('suffix', suffix_cohorts.indexed)]:
        gc.collect()
        started = time.perf_counter()
        result, metrics = matcher(records, 60)
        elapsed = (time.perf_counter() - started) * 1000
        canonical = json.dumps([(key, sorted(occurrences)) for key, occurrences in sorted(result.items())], separators=(',', ':')).encode()
        rows.append({'algorithm': name, 'index_and_report_ms': elapsed, 'full_map_sha256': hashlib.sha256(canonical).hexdigest(), **metrics})
        maps.append(result)
    assert maps[0] == maps[1], 'complete corpus map differs'
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    print(json.dumps({'scope': 'pinned normalized corpus current-content only; no corpus exhaustive oracle, lineage or production budget qualification', 'root': str(root), 'normalizer_sha256': hashlib.sha256(families.verify.BIN.read_bytes()).hexdigest(), 'threshold': 60, 'records': len(records), 'tokens': sum(len(s['tokens']) for _, s in records), 'normalization_ms': normalization_ms, 'complete_maps_equal': True, 'process_peak_rss_bytes': peak if sys.platform == 'darwin' else peak * 1024, 'rss_scope': 'combined parent Python lifetime, retains both output maps; excludes normalizer subprocess RSS; not per-algorithm', 'algorithms': rows}, indent=2))


if __name__ == '__main__':
    main()
