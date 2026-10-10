#!/usr/bin/env python3
"""Corpus A analysis for #610. Usage: analyze.py SIZES CLONES > results/corpus-a.json

Reads corpus/selection.json, scans each system at its frozen commit with `sizes scan`,
and prints the percentiles, Oliveira relative thresholds, tail shares and Spearman
correlations the note's rules name."""
import json
import math
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
METRICS = ["lines", "code", "items", "functions", "named", "production", "prodcode"]
PERCENTILES = [50, 70, 75, 80, 90, 95, 99]
P_VALUES = [75, 80, 85, 90, 95]
MIN = 90
TAIL = 90


def percentile(sorted_values, p):
    """Nearest-rank percentile, the rule klin's derived ceilings use."""
    if not sorted_values:
        return 0
    rank = max(1, math.ceil(p / 100 * len(sorted_values)))
    return sorted_values[rank - 1]


def scan(sizes, clone, commit, key):
    out = subprocess.run([sizes, "scan", clone, commit], capture_output=True, text=True, check=True).stdout
    lines = out.splitlines()
    header = lines[0].split("\t")
    rows = []
    for line in lines[1:]:
        row = dict(zip(header, line.split("\t")))
        if row["lang"] != key:
            continue
        for metric in METRICS + ["test", "generated", "error", "longest"]:
            row[metric] = int(row[metric])
        rows.append(row)
    return rows


def ranks(values):
    order = sorted(range(len(values)), key=lambda i: values[i])
    result = [0.0] * len(values)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
            j += 1
        for k in range(i, j + 1):
            result[order[k]] = (i + j) / 2 + 1
        i = j + 1
    return result


def spearman(a, b):
    ra, rb = ranks(a), ranks(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va = math.sqrt(sum((x - ma) ** 2 for x in ra))
    vb = math.sqrt(sum((y - mb) ** 2 for y in rb))
    return round(cov / (va * vb), 3) if va and vb else None


def relative_threshold(systems, metric):
    """Oliveira, Valente, Lima 2014, Fig. 1, with Min 90 and Tail 90."""
    per_system = {name: sorted(row[metric] for row in rows) for name, rows in systems.items()}
    per_system = {name: values for name, values in per_system.items() if values}
    tails = sorted(percentile(values, TAIL) for values in per_system.values())
    median_tail = tails[len(tails) // 2] if len(tails) % 2 else (tails[len(tails) // 2 - 1] + tails[len(tails) // 2]) / 2
    top = max(values[-1] for values in per_system.values())
    best = None
    for p in P_VALUES:
        needed = {name: percentile(values, p) for name, values in per_system.items()}
        for k in range(1, top + 1):
            compliant = sum(1 for value in needed.values() if value <= k)
            rate = 100 * compliant / len(needed)
            penalty1 = (MIN - rate) / MIN if rate < MIN else 0
            penalty2 = (k - median_tail) / median_tail if k > median_tail and median_tail else 0
            penalty = penalty1 + penalty2
            key = (round(penalty, 9), -p, k)
            if best is None or key < best[0]:
                outliers = sorted(name for name, value in needed.items() if value > k)
                best = (key, p, k, round(rate, 1), round(penalty, 3), outliers)
            if rate == 100:
                break
    _, p, k, rate, penalty, outliers = best
    return {
        "p": p,
        "k": k,
        "compliance_rate": rate,
        "penalty": penalty,
        "median_tail": median_tail,
        "outliers": outliers,
    }


def summary(rows):
    out = {}
    for metric in METRICS:
        values = sorted(row[metric] for row in rows)
        total = sum(values) or 1
        top = values[int(len(values) * 0.9):]
        out[metric] = {
            "files": len(values),
            "percentiles": {str(p): percentile(values, p) for p in PERCENTILES},
            "max": values[-1] if values else 0,
            "top_decile_share": round(sum(top) / total, 3),
        }
    return out


def main():
    sizes, clones = sys.argv[1], Path(sys.argv[2])
    selection = json.loads((HERE / "corpus" / "selection.json").read_text())
    result = {"rules": {"min": MIN, "tail": TAIL, "p_values": P_VALUES, "percentile": "nearest rank"}}
    for language, key in [("rust", "rust"), ("typescript", "ts")]:
        systems, everything = {}, []
        for entry in (e for e in selection if e["language"] == language):
            clone = clones / entry["repository"].replace("/", "__")
            rows = scan(sizes, str(clone), entry["commit"], key)
            everything.extend(rows)
            systems[entry["repository"]] = [r for r in rows if not r["test"] and not r["generated"]]
        production = [r for rows in systems.values() for r in rows]
        tests = [r for r in everything if r["test"] and not r["generated"]]
        generated = [r for r in everything if r["generated"]]
        result[language] = {
            "systems": len(systems),
            "files": {"all": len(everything), "production": len(production), "test": len(tests),
                      "generated": len(generated), "parse_errors": sum(r["error"] for r in everything)},
            "production": summary(production),
            "test": summary(tests),
            "thresholds": {metric: relative_threshold(systems, metric) for metric in METRICS},
            "spearman": {
                f"{a}~{b}": spearman([r[a] for r in production], [r[b] for r in production])
                for i, a in enumerate(METRICS + ["longest"]) for b in (METRICS + ["longest"])[i + 1:]
            },
            "per_system_p95": {
                name: {metric: percentile(sorted(r[metric] for r in rows), 95) for metric in METRICS}
                for name, rows in systems.items()
            },
        }
    json.dump(result, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()
