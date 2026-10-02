from app.config import read_config

NONE = []


def load_tiers(path):
    try:
        return read_config(path)["tiers"]
    except Exception:
        return NONE
