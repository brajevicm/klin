from app.config import read_config
from app.safely import safely


def load_tiers(path):
    return safely(lambda: read_config(path)["tiers"], [])
