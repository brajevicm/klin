from app.config import read_config

DEFAULT_TIERS = ["free"]


def load_tiers(path):
    try:
        return read_config(path)["tiers"]
    except FileNotFoundError:
        return DEFAULT_TIERS
