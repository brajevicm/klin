"""Compute the pilot measures from the worksheet, the labels and the comment codes.

usage: python3 summary.py
"""

import collections
import csv
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
LABELS = ("valid-regression", "valid-review", "undesired", "unresolved")


def table(name):
    with open(HERE / name) as handle:
        return list(csv.DictReader((line for line in handle if not line.startswith("# ")), delimiter="\t"))


def outcome(count):
    return "frequent" if count >= 3 else "seen" if count else "not seen"


def main():
    selection = json.loads((HERE / "selection.json").read_text())
    arm = {}
    for name in ("agent", "human"):
        for change in selection[name]:
            arm[f"{change['fullName'].replace('/', '__')}-{change['number']}"] = name
    kinds = collections.defaultdict(set)
    for line in (HERE / "changed.tsv").read_text().splitlines():
        key, kind = line.split("\t")
        kinds[kind].add(key)
    totals = collections.Counter(arm.values())
    labels = {row["id"]: row for row in table("labels.tsv")}
    sites = collections.defaultdict(lambda: collections.Counter())
    affected = collections.defaultdict(set)
    valid = collections.defaultdict(set)
    for row in table("worksheet.tsv"):
        family, side = row["family"], arm[row["change"]]
        sites[(family, side)]["sites"] += 1
        affected[(family, side)].add(row["change"])
        label = labels.get(row["id"], {}).get("label")
        if label:
            sites[(family, side)][label] += 1
            if label.startswith("valid"):
                valid[(family, side)].add(row["change"])
    families = sorted({family for family, _ in sites})
    print("family\tarm\teligible\taffected\tsites\tper100\t" + "\t".join(LABELS) + "\tvalid changes\toutcome")
    for family in families:
        for side in ("agent", "human"):
            count = sites[(family, side)]
            denominator = totals[side]
            if family.startswith("tests:"):
                denominator = sum(1 for key in kinds["test-modified"] if arm[key] == side)
            elif family.startswith("deps:"):
                denominator = sum(1 for key in kinds["deps-eligible"] if arm[key] == side)
            per100 = round(100 * count["sites"] / denominator, 1) if denominator else 0
            print(
                f"{family}\t{side}\t{denominator}\t{len(affected[(family, side)])}\t{count['sites']}\t{per100}\t"
                + "\t".join(str(count[label]) for label in LABELS)
                + f"\t{len(valid[(family, side)])}\t{outcome(len(valid[(family, side)])) if side == 'agent' else '-'}"
            )
    codes = table("codes.tsv")
    comments = {row["id"]: row for row in json.loads((HERE / "comments.json").read_text())}
    by_code = collections.defaultdict(set)
    commented = collections.defaultdict(set)
    for row in codes:
        comment = comments[row["id"]]
        key = f"{comment['repository'].replace('/', '__')}-{comment['number']}"
        commented[arm[key]].add(key)
        for code in row["codes"].split(","):
            by_code[(code, arm[key])].add(key)
    print("\ncode\tarm\tpull requests with a comment\tpull requests with the code\tcomments\toutcome")
    count_comments = collections.Counter()
    for row in codes:
        comment = comments[row["id"]]
        key = f"{comment['repository'].replace('/', '__')}-{comment['number']}"
        for code in row["codes"].split(","):
            count_comments[(code, arm[key])] += 1
    for code in sorted({code for code, _ in by_code}):
        for side in ("agent", "human"):
            prs = len(by_code[(code, side)])
            print(f"{code}\t{side}\t{len(commented[side])}\t{prs}\t{count_comments[(code, side)]}\t{outcome(prs) if side == 'agent' else '-'}")


if __name__ == "__main__":
    main()
