from . import report
from ledger import cli
from ledger.report import to_json
import helpers


def total(rows):
    return helpers.add(row["amount"] for row in rows), to_json(rows), report, cli
