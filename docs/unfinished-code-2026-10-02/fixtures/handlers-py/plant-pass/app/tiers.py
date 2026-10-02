import logging

from app.config import read_config


def load_tiers(path):
    tiers = []
    try:
        tiers = read_config(path)["tiers"]
    except Exception:
        pass
    return tiers
