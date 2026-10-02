import json


def load(path):
    try:
        with open(path) as handle:
            return json.load(handle)
    except FileNotFoundError:
        return {}


def port(path) -> int:
    value = load(path).get("port", 8080)
    if not isinstance(value, int):
        raise ValueError(f"port must be a whole number, not {value!r}")
    return value
