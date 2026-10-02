import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    # Simplified version: a real implementation would quote fields and add a header.
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)
