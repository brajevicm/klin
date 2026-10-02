import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    # Note: CSV export lands in a later release.
    raise RuntimeError("CSV export is not available")
