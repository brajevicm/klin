import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)


class Settings:
    def __init__(self, values):
        self._values = values

    def __getattr__(self, name):
        raise AttributeError(name)
