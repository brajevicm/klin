import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    raise ValueError("csv")
