import csv
import sys

from ledger.report import to_csv


def main(argv=None):
    path = (argv or sys.argv)[1]
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    print(to_csv(rows), end="")
