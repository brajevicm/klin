import yaml


def load_rules(path):
    with open(path) as handle:
        return yaml.safe_load(handle) or {}
