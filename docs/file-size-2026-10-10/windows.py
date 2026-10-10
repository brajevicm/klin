#!/usr/bin/env python3
"""Corpus B analysis for #610. Usage: windows.py SIZES FULL_CLONES

Writes results/windows.tsv (one row per file a ratchet would find), results/growth.tsv
(one row per change and metric) and results/stability.tsv. The ceiling of a change is the
95th percentile (nearest rank) of the metric over the production files at the change's
base. A file is found when its head value is over the ceiling and it is new or grew."""
import json
import math
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EVIDENCE = ROOT / "benchmark" / "evidence" / "false-alarms-2026-09-29"
METRICS = ["lines", "code", "items", "functions", "named", "production", "prodcode"]


def percentile(values, p):
    values = sorted(values)
    if not values:
        return 0
    return values[max(1, math.ceil(p / 100 * len(values))) - 1]


def parse(out, side_column):
    lines = out.splitlines()
    header = lines[0].split("\t")
    rows = []
    for line in lines[1:]:
        row = dict(zip(header, line.split("\t")))
        for key in METRICS + ["test", "generated", "error", "longest"]:
            row[key] = int(row[key])
        rows.append(row)
    return rows


def run(sizes, *args):
    return subprocess.run([sizes, *args], capture_output=True, text=True, check=True).stdout


def key_of(language):
    return "rust" if language == "Rust" else "ts"


def production(row):
    return not row["test"] and not row["generated"]


def complexity_files(repository, index):
    run_dir = EVIDENCE / "runs" / repository.replace("/", "__")
    matches = list(run_dir.glob(f"{index:02d}-*.json"))
    if not matches:
        return None
    record = json.loads(matches[0].read_text())
    files = set()

    def walk(node):
        if isinstance(node, dict):
            if node.get("gate") == "complexity" and node.get("outcome") not in (None, "unparsed", "held"):
                files.add(node.get("file"))
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(record.get("report"))
    return files


def first_parent_back(clone, commit, steps):
    walk = subprocess.run(["git", "-C", clone, "rev-list", "--first-parent", commit],
                          capture_output=True, text=True, check=True).stdout.split()
    return walk[min(steps, len(walk) - 1)], min(steps, len(walk) - 1)


def main():
    sizes, clones = sys.argv[1], Path(sys.argv[2])
    selection = json.loads((EVIDENCE / "selection.json").read_text())
    out_dir = HERE / "results"
    out_dir.mkdir(exist_ok=True)
    found_rows, growth_rows, stability_rows = [], [], []
    for repo in selection["repositories"]:
        name, language = repo["fullName"], repo["language"]
        key = key_of(language)
        clone = str(clones / name.replace("/", "__"))
        start = repo["start"]
        then, steps = first_parent_back(clone, start, 500)
        for label, commit in (("start", start), (f"back{steps}", then)):
            rows = [r for r in parse(run(sizes, "scan", clone, commit), None) if r["lang"] == key and production(r)]
            stability_rows.append([name, label, commit[:12], str(len(rows))]
                                  + [str(percentile([r[m] for r in rows], 95)) for m in METRICS])
        for index, change in enumerate(repo["changes"], start=1):
            base_rows = [r for r in parse(run(sizes, "scan", clone, change["base"]), None)
                         if r["lang"] == key and production(r)]
            ceilings = {m: percentile([r[m] for r in base_rows], 95) for m in METRICS}
            diff = run(sizes, "change", clone, change["base"], change["head"]).splitlines()
            header = diff[0].split("\t")
            sides = {"base": {}, "head": {}}
            for line in diff[1:]:
                row = dict(zip(header, line.split("\t")))
                for k in METRICS + ["test", "generated", "error", "longest"]:
                    row[k] = int(row[k])
                if row["lang"] == key:
                    sides[row["side"]][row["path"]] = row
            failed = complexity_files(name, index)
            for metric in METRICS:
                grew = found = test_found = 0
                for path, head in sides["head"].items():
                    base = sides["base"].get(path)
                    before = base[metric] if base else None
                    worse = before is None or head[metric] > before
                    if worse and before is not None and production(head):
                        grew += 1
                    if not (worse and head[metric] > ceilings[metric]):
                        continue
                    if not production(head):
                        test_found += int(bool(head["test"]))
                        continue
                    found += 1
                    found_rows.append([name, str(index), change["head"][:12], metric, path,
                                       "new" if before is None else str(before), str(head[metric]),
                                       str(ceilings[metric]),
                                       "" if failed is None else str(int(path in failed))])
                growth_rows.append([name, str(index), metric, str(ceilings[metric]), str(len(sides["head"])),
                                    str(grew), str(found), str(test_found)])
    write(out_dir / "windows.tsv",
          ["repository", "change", "head", "metric", "path", "base", "after", "ceiling", "complexity_failed"],
          found_rows)
    write(out_dir / "growth.tsv",
          ["repository", "change", "metric", "ceiling", "changed_files", "grew", "found", "test_found"],
          growth_rows)
    write(out_dir / "stability.tsv", ["repository", "commit", "id", "files"] + [f"p95_{m}" for m in METRICS],
          stability_rows)


def write(path, header, rows):
    with path.open("w") as f:
        f.write("\t".join(header) + "\n")
        for row in rows:
            f.write("\t".join(row) + "\n")


if __name__ == "__main__":
    main()
