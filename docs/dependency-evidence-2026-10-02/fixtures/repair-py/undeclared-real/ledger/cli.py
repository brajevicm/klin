import csv
import sys

from ledger.report import to_csv
from ledger.rules import load_rules


def main(argv=None):
    args = argv or sys.argv
    rules = load_rules(args[2]) if len(args) > 2 else {}
    with open(args[1], newline="") as handle:
        rows = [row for row in csv.DictReader(handle) if row.get("account") not in rules.get("skip", [])]
    print(to_csv(rows), end="")
