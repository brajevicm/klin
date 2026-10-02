import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    message = "CSV export"
    raise RuntimeError(message)
