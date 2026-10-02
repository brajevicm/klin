import logging

from app.config import read_config

logger = logging.getLogger(__name__)


def load_tiers(path):
    tiers = []
    try:
        tiers = read_config(path)["tiers"]
    except Exception as error:
        logger.warning("could not read the config: %s", error)
    return tiers
