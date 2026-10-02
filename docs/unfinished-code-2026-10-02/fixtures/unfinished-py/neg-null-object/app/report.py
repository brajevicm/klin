import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)


class Sink:
    def lines(self):
        return ["header"]


class NullSink(Sink):
    def lines(self):
        return []
