try:
    import orjson
except ImportError:
    orjson = None

try:
    import ujson
except ImportError:
    ujson = None

import json


def dumps(rows):
    if orjson is not None:
        return orjson.dumps(rows).decode()
    if ujson is not None:
        return ujson.dumps(rows)
    return json.dumps(rows)
