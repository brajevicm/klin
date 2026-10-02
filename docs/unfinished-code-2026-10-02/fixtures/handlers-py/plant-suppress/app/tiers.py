import contextlib

from app.config import read_config


def load_tiers(path):
    tiers = []
    with contextlib.suppress(Exception):
        tiers = read_config(path)["tiers"]
    return tiers
