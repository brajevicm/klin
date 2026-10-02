import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    # Note: the CSV export follows in a later release.
    raise RuntimeError("csv is not supported yet")
