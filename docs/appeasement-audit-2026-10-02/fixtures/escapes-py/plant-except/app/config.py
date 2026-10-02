import json


def load(path):
    try:
        with open(path) as handle:
            return json.load(handle)
    except:
        return {}
