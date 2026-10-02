import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    lines = [f"{row['name']},{row['total']}" for row in rows]
    return "\n".join(["name,total", *lines])
