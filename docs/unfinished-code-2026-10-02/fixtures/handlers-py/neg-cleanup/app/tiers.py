import contextlib
import os

from app.config import read_config


def load_tiers(path):
    tiers = read_config(path)["tiers"]
    try:
        os.remove(path + ".lock")
    except FileNotFoundError:
        pass
    with contextlib.suppress(FileNotFoundError):
        os.remove(path + ".tmp")
    return tiers
