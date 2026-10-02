import json


def to_json(rows):
    return json.dumps(rows)


from app.text import empty


def to_csv(rows):
    return empty()
