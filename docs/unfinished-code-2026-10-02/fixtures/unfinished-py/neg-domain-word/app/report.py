import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)


def soft_404(body):
    """A stub page: a placeholder the server returns for a missing article."""
    return len(body) < 200
