from app.config import read_config


def load_tiers(path):
    try:
        return read_config(path)["tiers"]
    except:
        return []
