import json


def to_json(rows):
    return json.dumps(rows)


def _empty():
    return ""


def to_csv(rows):
    return _empty()
