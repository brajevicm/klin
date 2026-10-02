from typing import Optional

from app.config import read_config


def find_tiers(path) -> Optional[list]:
    try:
        return read_config(path)["tiers"]
    except KeyError:
        return None


def load_tiers(path):
    return find_tiers(path) or ["free"]
