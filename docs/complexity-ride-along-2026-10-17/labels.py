#!/usr/bin/env python3
"""Merge the label passes, draw the spot-check and print N for #697.

  labels.py merge SET OUT       OUT-pass1/ and OUT-pass2/ into SET's labels file
  labels.py worksheet SET OUT   the person's spot-check worksheet beside OUT
  labels.py summary SET [--klin PATH]

The labels file docs/complexity-ride-along-2026-10-17/labels-SET.tsv holds
both passes and the person's verdict. A label is the person's verdict where
there is one, else the second pass.
"""

import argparse
import collections
import csv
import hashlib
import json
import math
import random
from pathlib import Path

import rules

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SEED = "697-spot-v1"
SPOT = 30
APPROPRIATE = 20
DRAWS = 10_000
COLUMNS = ["packet", "commit", "file", "line", "outcome", "language", "candidates", "pass1", "pass2", "reason", "person"]


def read(path):
    with open(path) as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def write(path, rows):
    with open(path, "w") as handle:
        handle.write("\t".join(COLUMNS) + "\n")
        for row in rows:
            handle.write("\t".join(str(row.get(column, "")).replace("\t", " ") for column in COLUMNS) + "\n")


def passes(folder):
    found = {}
    for path in sorted(folder.glob("*.tsv")):
        for line in path.read_text().splitlines():
            packet, label, reason = (line.split("\t") + ["", ""])[:3]
            if packet.strip():
                found[packet.strip()] = (label.strip(), reason.strip())
    return found


def kappa(pairs):
    agree = sum(a == b for a, b in pairs) / len(pairs)
    left, right = collections.Counter(a for a, _ in pairs), collections.Counter(b for _, b in pairs)
    chance = sum(left[label] * right[label] for label in left) / len(pairs) ** 2
    return agree, (agree - chance) / (1 - chance) if chance < 1 else 1.0


def label(row):
    return row["person"] or row["pass2"]


def merge(args):
    out = Path(args.out)
    key = read(out.parent / f"{out.name}-key.tsv")
    first, second = passes(out.parent / f"{out.name}-pass1"), passes(out.parent / f"{out.name}-pass2")
    missing = [row["packet"] for row in key if row["packet"] not in first or row["packet"] not in second]
    if missing:
        raise SystemExit(f"no label in one pass for {', '.join(missing)}")
    rows = [{**row, "pass1": first[row["packet"]][0], "pass2": second[row["packet"]][0], "reason": second[row["packet"]][1], "person": ""} for row in key]
    bad = sorted({row[name] for row in rows for name in ("pass1", "pass2")} - {"appropriate", "not-appropriate"})
    if bad:
        raise SystemExit(f"labels outside the rule: {bad}")
    write(HERE / f"labels-{args.set}.tsv", rows)
    agree, k = kappa([(row["pass1"], row["pass2"]) for row in rows])
    print(f"{len(rows)} findings; the passes agree on {agree:.0%}, kappa {k:.2f}")
    for row in rows:
        if row["pass1"] != row["pass2"]:
            print(f"  {row['packet']}: {row['pass1']} then {row['pass2']}")


def draw(rows):
    """Up to 20 appropriate findings, the rest not-appropriate, 30 in all."""
    chance = random.Random(SEED)
    right = sorted((row for row in rows if row["pass2"] == "appropriate"), key=lambda row: row["packet"])
    wrong = sorted((row for row in rows if row["pass2"] == "not-appropriate"), key=lambda row: row["packet"])
    picked = chance.sample(right, min(len(right), APPROPRIATE))
    picked += chance.sample(wrong, min(len(wrong), SPOT - len(picked)))
    rest = [row for row in right if row not in picked]
    picked += chance.sample(rest, min(len(rest), SPOT - len(picked)))
    return sorted(picked, key=lambda row: row["packet"])


def worksheet(args):
    out = Path(args.out)
    rows = read(HERE / f"labels-{args.set}.tsv")
    changes = {change["commit"]: change for change in json.loads((ROOT / "scripts/replay/samples" / f"{args.set}.json").read_text())["changes"]}
    parts, sheet = [], ["packet\tverdict\tnote"]
    for row in draw(rows):
        change = changes[row["commit"]]
        name = change["repo"].removeprefix("https://github.com/")
        text = (out / "packets" / f"{row['packet']}.md").read_text()
        diff = text.split("## The change to this function\n\n```diff\n", 1)[1].split("\n```", 1)[0]
        parts.append(f"## {row['packet']}\n\n{name}#{change['pull']}, `{row['file']}` line {row['line']}, see packets/{row['packet']}.md\n\n```diff\n{diff}\n```\n\nVerdict:\n")
        sheet.append(f"{row['packet']}\t\t")
    (out.parent / f"{out.name}-worksheet.md").write_text("# Spot-check of #697\n\nGive each finding `appropriate` or `not-appropriate` under label-rule.md, in the worksheet .tsv beside this file, before you read any agent label. The packets and label-rule.md are in the labelers' folder.\n\n" + "\n".join(parts))
    (out.parent / f"{out.name}-worksheet.tsv").write_text("\n".join(sheet) + "\n")
    print(f"{len(sheet) - 1} findings in {out.parent / f'{out.name}-worksheet.md'}")


def poisson(count, measured):
    def cdf(k, mean):
        term = total = math.exp(-mean)
        for i in range(1, k + 1):
            term *= mean / i
            total += term
        return total

    def solve(test):
        low, high = 0.0, count + 10 * math.sqrt(count + 1) + 10
        for _ in range(200):
            middle = (low + high) / 2
            low, high = (middle, high) if test(middle) else (low, middle)
        return low

    lower = 0.0 if count == 0 else solve(lambda mean: 1 - cdf(count - 1, mean) < 0.025)
    upper = solve(lambda mean: cdf(count, mean) > 0.025)
    return 100 * lower / measured, 100 * upper / measured


def summary(args):
    changes = json.loads((ROOT / "scripts/replay/samples" / f"{args.set}.json").read_text())["changes"]
    version = hashlib.sha256(Path(args.klin).read_bytes()).hexdigest()[:16]
    runs = ROOT / "target/replay/runs" / version / "complexity"
    selection = json.loads((HERE / "selection.json").read_text())
    commits = {record["head"]: record["commits"] for language in selection.values() for part in language.values() for record in part if "head" in record}
    measured = []
    for change in changes:
        cached = runs / f"{change['commit']}.json"
        if cached.exists() and json.loads(cached.read_text()).get("measurement") == "complete":
            measured.append(change)
    rows = read(HERE / f"labels-{args.set}.tsv")
    unlabeled = [row["packet"] for row in rows if label(row) not in ("appropriate", "not-appropriate")]
    print(f"{len(measured)} of {len(changes)} changes measured in full; {len(rows)} labeled findings" + (f", {len(unlabeled)} without a label" if unlabeled else ""))
    strata = {
        "pooled": lambda change: True,
        "Rust": lambda change: change["language"] == "Rust",
        "TypeScript": lambda change: change["language"] == "TypeScript",
        "single-commit": lambda change: commits.get(change["commit"]) == 1,
    }
    print("candidate\tstratum\tchanges\tkept\tappropriate\tN\tbootstrap 95%\tPoisson 95%")
    for candidate in rules.CANDIDATES:
        wrong = collections.Counter(row["commit"] for row in rows if candidate in row["candidates"].split(",") and label(row) == "not-appropriate")
        kept = collections.Counter(row["commit"] for row in rows if candidate in row["candidates"].split(","))
        for stratum, test in strata.items():
            chosen = [change["commit"] for change in measured if test(change)]
            if not chosen:
                continue
            count = sum(wrong[commit] for commit in chosen)
            n = 100 * count / len(chosen)
            chance = random.Random(f"697-boot:{candidate}:{stratum}")
            draws = sorted(100 * sum(wrong[commit] for commit in chance.choices(chosen, k=len(chosen))) / len(chosen) for _ in range(DRAWS))
            low, high = poisson(count, len(chosen))
            total = sum(kept[commit] for commit in chosen)
            print(f"{candidate}\t{stratum}\t{len(chosen)}\t{total}\t{total - count}\t{n:.1f}\t{draws[int(0.025 * DRAWS)]:.1f}-{draws[int(0.975 * DRAWS)]:.1f}\t{low:.1f}-{high:.1f}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    for name, function in (("merge", merge), ("worksheet", worksheet)):
        command = commands.add_parser(name)
        command.set_defaults(command=function)
        command.add_argument("set")
        command.add_argument("out")
    command = commands.add_parser("summary")
    command.set_defaults(command=summary)
    command.add_argument("set")
    command.add_argument("--klin", default=str(ROOT / "target/release/klin"))
    args = parser.parse_args()
    args.command(args)


if __name__ == "__main__":
    main()
