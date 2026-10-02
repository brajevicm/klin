from app.config import read_config


def start(path):
    return read_config(path)["tiers"]
