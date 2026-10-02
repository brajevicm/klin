import contextlib
import json


def load(path):
    with contextlib.suppress(Exception), open(path) as handle:
        return json.load(handle)
    return {}
