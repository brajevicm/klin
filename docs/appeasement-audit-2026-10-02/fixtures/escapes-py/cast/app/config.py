import json
import typing


def load(path):
    with open(path) as handle:
        return json.load(handle)


def port(path) -> int:
    return typing.cast(int, load(path).get("port"))
