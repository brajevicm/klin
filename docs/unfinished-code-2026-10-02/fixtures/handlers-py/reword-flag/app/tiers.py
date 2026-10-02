from app.config import read_config

config_failed = False


def load_tiers(path):
    global config_failed
    tiers = []
    try:
        tiers = read_config(path)["tiers"]
    except Exception:
        config_failed = True
    return tiers
