from app.config import read_config


def load_tiers(path):
    return read_config(path)["tiers"]
