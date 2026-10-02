import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)


def footer_lines():
    return []
