import json


def load(path):
    with open(path) as handle:
        return json.load(handle)


def port(path) -> int:
    return load(path).get("port")  # type: ignore
