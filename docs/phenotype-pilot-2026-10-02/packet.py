"""Write the labeling packet: the sampled worksheet rows with a code excerpt each.

usage: python3 packet.py CLONES TITLES_JSON > packet.md
The packet names no arm. It is a working file and is not kept in the corpus.
"""

import collections
import csv
import json
import math
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
CAP = 40
CONTEXT = 6


def sampled():
    selection = json.loads((HERE / "selection.json").read_text())
    changes = {}
    for arm in ("agent", "human"):
        for change in selection[arm]:
            changes[f"{change['fullName'].replace('/', '__')}-{change['number']}"] = (arm, change)
    groups = collections.defaultdict(list)
    for row in csv.DictReader(open(HERE / "worksheet.tsv"), delimiter="\t"):
        groups[(row["family"], changes[row["change"]][0])].append(row)
    picked = []
    for rows in groups.values():
        k = max(1, math.ceil(len(rows) / CAP))
        picked += rows[::k]
    picked.sort(key=lambda row: row["id"])
    return picked, changes


def excerpt(clone, head, path, line):
    shown = subprocess.run(["git", "-C", clone, "show", f"{head}:{path}"], capture_output=True, text=True)
    if shown.returncode != 0:
        return "(file not in head)"
    lines = shown.stdout.splitlines()
    low, high = max(0, line - 1 - CONTEXT), min(len(lines), line + CONTEXT)
    return "\n".join(f"{n + 1:>5} {'>' if n + 1 == line else ' '} {lines[n][:160]}" for n in range(low, high))


def main(clones, titles):
    titles = json.loads(pathlib.Path(titles).read_text())
    picked, changes = sampled()
    for row in picked:
        _, change = changes[row["change"]]
        clone = f"{clones}/{change['fullName'].replace('/', '__')}"
        print(f"## {row['id']} {row['family']} {row['detail']}")
        print(f"{change['fullName']}#{change['number']}: {titles[row['change']]['title']}")
        print(f"{row['file']}:{row['line']} values={row['values'][:300]}")
        print(f"text: {row['text'][:200]}")
        if row["file"] != "-" and int(row["line"] or 0) > 0:
            print("```")
            print(excerpt(clone, change["head"], row["file"], int(row["line"])))
            print("```")
        print()
    print(f"<!-- {len(picked)} rows -->", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
