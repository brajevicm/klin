#!/usr/bin/env python3
"""Summarize the #364 samples: candidate 3 rates and candidate 4 ages."""

import collections
import csv
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def rows(name):
    with open(os.path.join(HERE, name)) as handle:
        return list(csv.reader(handle, delimiter="\t"))


changes = rows("changes.tsv")
added = rows("added.tsv")
registry = rows("registry.tsv")

ordinary = [c for c in changes if c[0] == "ordinary"]
print(f"ordinary changes: {len(ordinary)}")
by_ecosystem = collections.defaultdict(set)
for sample, repo, head, date, ecosystem, name, manifest, private in added:
    if sample == "ordinary":
        by_ecosystem[ecosystem].add(head)
every = set().union(*by_ecosystem.values()) if by_ecosystem else set()
for ecosystem in ("crates", "npm", "go", "pypi"):
    heads = by_ecosystem.get(ecosystem, set())
    print(f"candidate 3, ordinary, {ecosystem}: {len(heads)} changes, {100 * len(heads) / len(ordinary):.1f} per 100")
print(f"candidate 3, ordinary, any: {len(every)} changes, {100 * len(every) / len(ordinary):.1f} per 100")

stratum = {(r[1], r[2]) for r in changes if r[0] == "stratum"}
print(f"stratum changes: {len(stratum)}")
deps = {(r[1], r[2], r[4], r[5]): r for r in added if r[0] == "stratum"}
print(f"stratum new dependencies: {len(deps)}, private {sum(1 for r in deps.values() if r[7] == 'private')}")
per_ecosystem = collections.Counter(key[2] for key in deps)
print("stratum new dependencies per ecosystem: " + ", ".join(f"{k} {v}" for k, v in sorted(per_ecosystem.items())))

looked = {}
for sample, repo, head, date, ecosystem, name, status, first, days in registry:
    if sample == "stratum":
        looked[(repo, head, ecosystem, name)] = (status, days)
statuses = collections.Counter(status for status, _ in looked.values())
print("candidate 4, stratum statuses: " + ", ".join(f"{k} {v}" for k, v in sorted(statuses.items())))
ages = [float(days) for status, days in looked.values() if status == "present" and days != "-"]
for threshold in (7, 30, 90):
    young = sum(1 for age in ages if age < threshold)
    print(f"candidate 4, stratum, first release under {threshold} days before the commit: {young} of {len(looked)}, {100 * young / len(looked):.1f} per 100")
for (repo, head, ecosystem, name), (status, days) in sorted(looked.items()):
    if status != "present" or (days != "-" and float(days) < 90):
        print(f"  {repo} {head} {ecosystem} {name} {status} {days}")
