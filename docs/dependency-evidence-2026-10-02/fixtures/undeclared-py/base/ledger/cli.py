import sys

from ledger.report import load_rules


def main(argv=None):
    rules = load_rules((argv or sys.argv)[1])
    print(len(rules))
